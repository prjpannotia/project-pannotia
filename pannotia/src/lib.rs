//! `pannotia`&mdash;handle bitstreams for [AGM32](https://www.agm-micro.com/) FPGAs, codenamed `rodinia`.

pub mod chips;
pub mod container;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
