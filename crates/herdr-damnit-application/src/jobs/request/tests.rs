use herdr_damnit_domain::Oid;

use super::*;

fn oid(text: &str) -> Oid {
    Oid::new(text)
}

#[test]
fn each_request_names_the_argv_it_runs() {
    assert_eq!(Request::Version.argv(), ["--version"]);
    assert_eq!(Request::Status.argv(), ["status", "--json"]);
    assert_eq!(
        Request::List("!done".to_string()).argv(),
        ["ls", "!done", "--json"]
    );
    assert_eq!(Request::Done.argv(), ["ls", "done", "--json"]);
    assert_eq!(Request::Show(oid("1a2")).argv(), ["show", "1a2", "--json"]);
    assert_eq!(Request::Log.argv(), ["log", "--json"]);
    assert_eq!(
        Request::Write(argv::stage_all()).argv(),
        ["add", "-A", "--json"]
    );
    assert_eq!(
        Request::Sync(SyncKind::Push, argv::push()).argv(),
        ["push", "--json"]
    );
}

#[test]
fn a_write_and_a_sync_ask_for_a_reread_and_a_read_does_not() {
    assert!(Request::Write(argv::stage_all()).rereads_after());
    assert!(Request::Sync(SyncKind::Push, argv::push()).rereads_after());
    assert!(!Request::Status.rereads_after());
    assert!(!Request::List("!done".to_string()).rereads_after());
    assert!(!Request::Version.rereads_after());
}

#[test]
fn a_read_supersedes_an_older_read_of_its_own_kind_and_nothing_else() {
    let open = Request::List("!done".to_string());
    assert!(Request::List("done".to_string()).supersedes(&open));
    assert!(Request::Show(oid("1")).supersedes(&Request::Show(oid("1"))));
    assert!(!Request::Show(oid("2")).supersedes(&Request::Show(oid("1"))));
    assert!(!Request::Status.supersedes(&open));
    assert!(!Request::Write(argv::stage_all()).supersedes(&Request::Write(argv::stage_all())));
    assert!(!Request::Version.supersedes(&Request::Version));
}
