
#[derive(RustcDecodable, Debug)]
pub struct Status {
    pub busy: bool,
    pub matempty: bool
}

#[derive(Debug)]
pub struct Printer {
    pub id : usize,
    pub fabid : usize,
    pub address : String,
    pub status : Status
}

impl Printer {
    pub fn new(fabid : usize, id : usize, address : String) -> Self {
        Printer {
            id: id,
            fabid: fabid,
            address: address,
            status: Status { busy: false, matempty: false }
        }
    }
}