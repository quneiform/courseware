fn main() {
    let domain = [Request::Read, Request::List, Request::Write];
    let codomain = [Handler::Query, Handler::Update];
    for inputs in 0..8 {
        let selected: Vec<_> = domain.iter().enumerate()
            .filter(|(i, _)| inputs & (1 << i) != 0).map(|(_, x)| *x).collect();
        let actual_image = image(&selected);
        for handler in codomain {
            assert_eq!(actual_image.iter().filter(|x| **x == handler).count(),
                usize::from(selected.iter().any(|x| route(*x) == handler)),
                "image of {selected:?} at {handler:?}");
        }
        for outputs in 0..4 {
            let allowed: Vec<_> = codomain.iter().enumerate()
                .filter(|(i, _)| outputs & (1 << i) != 0).map(|(_, x)| *x).collect();
            let actual_preimage = preimage(&allowed);
            for request in domain {
                assert_eq!(actual_preimage.iter().filter(|x| **x == request).count(),
                    usize::from(allowed.contains(&route(request))), "preimage of {allowed:?}");
            }
            assert_eq!(actual_image.iter().all(|x| allowed.contains(x)),
                selected.iter().all(|x| actual_preimage.contains(x)));
        }
    }
    println!("All 32 subset pairs passed.");
}
