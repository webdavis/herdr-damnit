use crossterm::event::KeyCode;

use crate::edit::tests::{character, list_of_one, press, refusing, serve, serve_reading, typed};

#[tokio::test]
async fn a_refused_write_reports_the_api_s_own_message_and_keeps_the_rows_on_screen() {
    let double = refusing().await;
    let mut list = list_of_one(1, &[]);

    let mut keys = vec![character('s')];
    keys.extend(typed("wenesday"));
    keys.push(KeyCode::Enter);
    let status = press(&double, &mut list, &keys).await;

    assert_eq!(
        status,
        "Invalid argument value: Unable to parse the due date"
    );
    assert_eq!(list.task_count(), 1);
    assert_eq!(list.selected_id(), Some("6X"));
}

#[tokio::test]
async fn a_takes_a_whole_quick_add_line_and_reports_the_task_todoist_made_of_it() {
    let double = serve("200 OK", r#"{"id":"7","content":"Pay rent","priority":4}"#).await;
    let mut list = list_of_one(1, &[]);

    let mut keys = vec![character('a')];
    keys.extend(typed("Pay rent tomorrow 9am p1 #Finances @home"));
    keys.push(KeyCode::Enter);
    let status = press(&double, &mut list, &keys).await;

    assert_eq!(
        double.writes(),
        [r#"POST /tasks/quick {"text":"Pay rent tomorrow 9am p1 #Finances @home"}"#.to_string()]
    );
    assert_eq!(status, "added Pay rent  0 open tasks");
}

#[tokio::test]
async fn l_toggles_a_label_on_and_off_from_the_picker() {
    let double = serve_reading(
        "200 OK",
        "null",
        &[(
            "/labels",
            r#"{"results":[{"name":"home"}],"next_cursor":null}"#,
        )],
    )
    .await;
    let mut list = list_of_one(1, &[]);

    // The label picker reads the account's labels, then `<CR>` writes the task's whole label
    // set with the one under the cursor added, and a second `<CR>` writes it back off.
    let status = press(
        &double,
        &mut list,
        &[character('l'), KeyCode::Enter, KeyCode::Enter],
    )
    .await;

    assert_eq!(
        double.writes(),
        [
            r#"POST /tasks/6X {"labels":["home"]}"#.to_string(),
            r#"POST /tasks/6X {"labels":[]}"#.to_string(),
        ]
    );
    assert_eq!(status, "@home off  0 open tasks");
}

#[tokio::test]
async fn l_with_no_labels_anywhere_says_so_instead_of_opening_an_empty_picker() {
    let double = serve_reading(
        "200 OK",
        "null",
        &[("/labels", r#"{"results":[],"next_cursor":null}"#)],
    )
    .await;
    let mut list = list_of_one(1, &[]);

    let status = press(&double, &mut list, &[character('l')]).await;

    assert_eq!(status, "no labels");
    assert_eq!(double.writes(), Vec::<String>::new());
}

#[tokio::test]
async fn m_moves_the_task_to_the_destination_picked() {
    let double = serve_reading(
        "200 OK",
        "null",
        &[(
            "/projects",
            r#"{"results":[{"id":"p1","name":"First"}],"next_cursor":null}"#,
        )],
    )
    .await;
    let mut list = list_of_one(1, &[]);

    let status = press(&double, &mut list, &[character('m'), KeyCode::Enter]).await;

    assert_eq!(
        double.writes(),
        [r#"POST /tasks/6X/move {"project_id":"p1"}"#.to_string()],
        "{:?}",
        double.requests()
    );
    assert_eq!(status, "moved to First  0 open tasks");
}

#[tokio::test]
async fn p_cycles_one_step_up_in_urgency_and_wraps_at_the_urgent_end() {
    for (from, sent, said) in [(1u8, 2u8, "p3"), (3, 4, "p1"), (4, 1, "p4")] {
        let double = serve("200 OK", "null").await;
        let mut list = list_of_one(from, &[]);

        let status = press(&double, &mut list, &[character('p')]).await;

        assert_eq!(
            double.writes(),
            [format!("POST /tasks/6X {{\"priority\":{sent}}}")],
            "from {from}"
        );
        assert_eq!(status, format!("{said}  0 open tasks"));
    }
}

#[tokio::test]
async fn s_sends_the_line_typed_as_the_natural_language_due_string() {
    let double = serve("200 OK", "null").await;
    let mut list = list_of_one(1, &[]);

    let mut keys = vec![character('s')];
    keys.extend(typed("every 2 weeks"));
    keys.push(KeyCode::Enter);
    let status = press(&double, &mut list, &keys).await;

    assert_eq!(
        double.writes(),
        [r#"POST /tasks/6X {"due_string":"every 2 weeks"}"#.to_string()]
    );
    assert_eq!(status, "due set  0 open tasks");
}

#[tokio::test]
async fn shift_x_reopens_the_task_under_the_cursor() {
    let double = serve("200 OK", "null").await;
    let mut list = list_of_one(1, &[]);

    let status = press(&double, &mut list, &[character('X')]).await;

    assert_eq!(double.writes(), ["POST /tasks/6X/reopen".to_string()]);
    assert_eq!(status, "reopened  0 open tasks");
}

#[tokio::test]
async fn x_closes_the_task_under_the_cursor() {
    let double = serve("200 OK", "null").await;
    let mut list = list_of_one(1, &[]);

    let status = press(&double, &mut list, &[character('x')]).await;

    assert_eq!(double.writes(), ["POST /tasks/6X/close".to_string()]);
    assert_eq!(status, "completed  0 open tasks");
}
