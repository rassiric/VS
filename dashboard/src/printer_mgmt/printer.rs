
#[derive(RustcDecodable, Debug)]
pub struct Status {
    pub busy: bool,
    pub matempty: bool,
    pub current_job: String
}

#[derive(Debug)]
pub struct Printer {
    pub id : usize,
    pub fabid : usize,
    pub address : String,
    pub reachable : bool,
    pub status : Status
}

impl Printer {
    pub fn new(fabid : usize, id : usize, address : String) -> Self {
        Printer {
            id: id,
            fabid: fabid,
            address: address,
            reachable: false,
            status: Status { busy: false, matempty: false, current_job: "".to_string() }
        }
    }
}