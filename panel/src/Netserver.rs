extern crate mio;


mod Netserver{
    use std::thread;
    use mio::*;
    struct Ns;

    impl Handler for Ns {

        type Timeout = usize;
        type Message = ();
    }
}
