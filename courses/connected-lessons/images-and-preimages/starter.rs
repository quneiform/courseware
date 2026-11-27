#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request { Read, List, Write }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler { Query, Update }

fn route(request: Request) -> Handler {
    match request {
        Request::Read | Request::List => Handler::Query,
        Request::Write => Handler::Update,
    }
}

// Return each handler reached by the selected requests exactly once.
fn image(selected: &[Request]) -> Vec<Handler> {
    todo!()
}

// Return every request whose route is allowed, without duplicates.
fn preimage(allowed: &[Handler]) -> Vec<Request> {
    todo!()
}
