
use git2::Repository;

#[(cfg(test)]
mod tests {
    #[test]
    fn test_print() {
        let repo = match Repository::open("/path/to/a/repo") {
            Ok(repo) => println!(repo),
            Err(e) => panic!("failed to open: {}", e),
        };
    }
}
