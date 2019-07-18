use super::BABNode;
use crate::hungarian::EdgeWeight;
use crate::{Assignment, Course, Participant};
use ndarray::Array1;
use crate::bab::NodeResult;

fn create_simple_problem() -> (Vec<Participant>, Vec<Course>) {
    // Idea: Course 1 or 2 must be cancelled, b/c otherwise, we don't have enough participants to fill all courses.
    // Course 1 will win due to Participant 5's choices, so Course 2 will be cancelled.
    (
        vec![
            Participant {
                index: 0,
                dbid: 0,
                name: String::from("Participant 0"),
                choices: vec![1, 2],
            },
            Participant {
                index: 1,
                dbid: 1,
                name: String::from("Participant 1"),
                choices: vec![0, 2],
            },
            Participant {
                index: 2,
                dbid: 2,
                name: String::from("Participant 2"),
                choices: vec![0, 1],
            },
            Participant {
                index: 3,
                dbid: 3,
                name: String::from("Participant 3"),
                choices: vec![0, 1],
            },
            Participant {
                index: 4,
                dbid: 4,
                name: String::from("Participant 4"),
                choices: vec![0, 2],
            },
            Participant {
                index: 5,
                dbid: 5,
                name: String::from("Participant 5"),
                choices: vec![1, 2],
            },
        ],
        vec![
            Course {
                index: 0,
                dbid: 0,
                name: String::from("Wanted Course 0"),
                num_max: 2,
                num_min: 2,
                instructors: vec![0],
            },
            Course {
                index: 1,
                dbid: 1,
                name: String::from("Okay Course 1"),
                num_max: 8,
                num_min: 2,
                instructors: vec![1],
            },
            Course {
                index: 2,
                dbid: 2,
                name: String::from("Boring Course 2"),
                num_max: 10,
                num_min: 2,
                instructors: vec![2],
            },
        ],
    )
}

#[test]
fn test_precompute_problem() {
    let (participants, courses) = create_simple_problem();

    let problem = super::precompute_problem(&courses, &participants);

    // check vector sizes
    let n = courses.iter().fold(0, |acc, c| acc + c.num_max);
    assert_eq!(problem.adjacency_matrix.dim().0, n);
    assert_eq!(problem.adjacency_matrix.dim().1, n);
    assert_eq!(problem.dummy_x.dim(), n);
    assert_eq!(problem.course_map.dim(), n);
    assert_eq!(problem.inverse_course_map.len(), courses.len());

    // Check references of courses
    for (i, c) in courses.iter().enumerate() {
        for j in 0..c.num_max {
            let base_column = problem.inverse_course_map[i];
            assert_eq!(
                problem.course_map[base_column + j],
                i,
                "Column {} should be mapped to course {}, as it is within {} columns after {}",
                base_column + j,
                i,
                c.num_max,
                base_column
            );
        }
    }

    // check adjacency matrix
    const WEIGHTS: [u16; 3] = [super::WEIGHT_OFFSET, super::WEIGHT_OFFSET-1, super::WEIGHT_OFFSET-2];
    for (x, p) in participants.iter().enumerate() {
        for y in 0..n {
            let choice = p.choices.iter().position(|c| *c == problem.course_map[y]);
            assert_eq!(
                problem.adjacency_matrix[(x, y)],
                match choice {
                    Some(c) => WEIGHTS[c],
                    None => 0,
                },
                "Edge weigth for participant {} with course place {} is not expected.",
                x,
                y
            );
        }
    }
    for x in participants.len()..n {
        for y in 0..n {
            assert_eq!(
                problem.adjacency_matrix[(x, y)],
                0,
                "Edge weigth for dummy participant {} with course place {} is not zero.",
                x,
                y
            );
        }
    }

    // check dummy_x
    for x in 0..participants.len() {
        assert!(!problem.dummy_x[x]);
    }
    for x in participants.len()..n {
        assert!(problem.dummy_x[x]);
    }
}

#[test]
fn test_babnode_sorting() {
    let node0 = BABNode {
        cancelled_courses: vec![],
        enforced_courses: vec![],
    };
    let node1 = BABNode {
        cancelled_courses: vec![0],
        enforced_courses: vec![],
    };
    assert!(node0 < node1);
    let node2 = BABNode {
        cancelled_courses: vec![],
        enforced_courses: vec![2],
    };
    assert!(node0 < node2);
    let node3 = BABNode {
        cancelled_courses: vec![1, 2],
        enforced_courses: vec![],
    };
    assert!(node1 < node3);
    assert!(node2 < node3);
    let node4 = BABNode {
        cancelled_courses: vec![],
        enforced_courses: vec![0, 1, 2],
    };
    assert!(node2 < node4);
    let node5 = BABNode {
        cancelled_courses: vec![0, 1],
        enforced_courses: vec![0, 1],
    };
    assert!(node4 < node5);
}

#[test]
fn test_check_feasibility() {
    let (participants, courses) = create_simple_problem();

    // A feasible assignment
    let assignment: Assignment = vec![0, 1, 1, 0, 0, 1];
    let course_instructors =
        ndarray::Array1::from_vec(vec![true, true, false, false, false, false]);
    let node = BABNode {
        cancelled_courses: vec![2],
        enforced_courses: vec![],
    };
    assert_eq!(
        super::check_feasibility(
            &courses,
            &participants,
            &assignment,
            &node,
            &course_instructors
        ),
        (true, false, None)
    );

    // Courses 1 & 2 have to few participants. Course 2 lacks more than Course 1.
    let assignment: Assignment = vec![0, 1, 2, 0, 0, 1];
    let course_instructors = ndarray::Array1::from_vec(vec![true, true, true, false, false, false]);
    let node = BABNode {
        cancelled_courses: vec![],
        enforced_courses: vec![],
    };
    assert_eq!(
        super::check_feasibility(
            &courses,
            &participants,
            &assignment,
            &node,
            &course_instructors
        ),
        (false, false, Some(2))
    );

    // Let's ignore that Participant 4 chose course 0. Participant 5 has been assigned to wrong course 0. We should
    // cancel course 2 to fill course 0 with its instructor.
    let assignment: Assignment = vec![0, 1, 2, 0, 1, 0];
    let course_instructors = ndarray::Array1::from_vec(vec![true, true, true, false, false, false]);
    let node = BABNode {
        cancelled_courses: vec![],
        enforced_courses: vec![0],
    };
    assert_eq!(
        super::check_feasibility(
            &courses,
            &participants,
            &assignment,
            &node,
            &course_instructors
        ),
        (false, true, Some(2))
    );
}

/// Testing helper function to check correctness of a feasible solution
fn check_assignment(
    courses: &Vec<Course>,
    participants: &Vec<Participant>,
    assignment: &Assignment,
    node: &BABNode) {

    // Check course instructor assignment
    let mut course_instructors = vec![false; participants.len()];
    for (c, course) in courses.iter().enumerate() {
        if !node.cancelled_courses.contains(&c) {
            for i in course.instructors.iter() {
                assert_eq!(
                    assignment[*i], c,
                    "Instructor {} of course {} is assigned to {}",
                    *i, c, assignment[*i]);
                course_instructors[*i] = true;
            }
        }
    }

    // Calculate course sizes
    let mut course_size = vec![0usize; courses.len()];
    for (p, c) in assignment.iter().enumerate() {
        if !course_instructors[p] {
            course_size[*c] += 1;
        }
    }

    // Check course sizes
    for (c, size) in course_size.iter().enumerate() {
        assert!(
            *size <= courses[c].num_max,
            "Maximum size violation for course {}: {} places, {} participants",
            c, courses[c].num_max, size);
        // Feasible solutions must not have wrong assigned participants
        if !node.cancelled_courses.contains(&c) {
            assert!(
                *size >= courses[c].num_min,
                "Minimum size violation for course {}: {} required, {} assigned",
                c, courses[c].num_min, size);
        }
    }

    // Check cancelled courses
    for c in node.cancelled_courses.iter() {
        assert_eq!(
            course_size[*c], 0, "Cancelled course {} has {} participants", *c, course_size[*c]);
    }

    // Feasible solutions must not have wrong assigned participants
    for (p, participant) in participants.iter().enumerate() {
        if !course_instructors[p] {
            assert!(
                participant.choices.contains(&assignment[p]),
                "Course {} of participant {} is none of their choices ({:?})",
                assignment[p], p, participant.choices
            );
        }
    }
}

#[test]
fn test_bab_node_simple() {
    // This test depends on `precompute_problem()`, `check_feasibility()` and `hungarian::hungarian_algorithm()`,
    // so if it fails, please check their test results first.

    let (participants, courses) = create_simple_problem();
    let problem = super::precompute_problem(&courses, &participants);
    let node = BABNode {
        cancelled_courses: vec![1],
        enforced_courses: vec![],
    };

    let result = super::run_bab_node(&courses, &participants, &problem, node.clone());

    match result {
        NodeResult::Feasible(assignment, score) => {
            print!("assignment: {:?}\n", assignment);
            check_assignment(&courses, &participants, &assignment, &node);
            assert!(score > participants.len() as u32 * (super::WEIGHT_OFFSET as u32 - 1));
        },
        x => panic!("Expected feasible result, got {:?}", x)
    }

}

// TODO test run_bab_node with large problem

// TODO test solve with simple problem

// TODO test solve with large problem
