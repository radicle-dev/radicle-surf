#[cfg(test)]
mod tests {
    use git2::Repository;

    #[test]
    fn test_print() {
        let repo = match Repository::open("/home/haptop/Developer/radicle-surf") {
            Ok(repo) => println!("{:#?}", repo),
            Err(e) => panic!("failed to open: {}", e),
        };
    }
}
