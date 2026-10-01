use herdr_damnit_domain::Oid;

pub struct Confirm {
    pub question: String,
    pub key: char,
    pub purpose: ConfirmPurpose,
}

pub enum ConfirmPurpose {
    Delete(Oid),
    Discard(Oid),
    QuitMidJob,
}
