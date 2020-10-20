use rocket::http::Status;

#[get("/live")]
pub fn live() -> Status {
    Status::Ok
}

#[get("/ready")]
pub fn ready() -> Status {
    Status::Ok
}
