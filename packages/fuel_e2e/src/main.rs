fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use some_macros::test_project_abigen;

    #[test]
    fn something() {
        test_project_abigen!(MyContract, "enum_encoding");
    }
}
