use crate::PackerParse;

impl PackerParse for bool {
    fn try_parse(s: &str) -> Result<Self, ()>
    where
        Self: Sized,
    {
        match s.to_ascii_lowercase().as_str() {
            "0" | "f" | "false" => Ok(false),
            "1" | "t" | "true" => Ok(true),
            _ => Err(()),
        }
    }
}

impl PackerParse for ::bitmux::InvertedBool {
    fn try_parse(s: &str) -> Result<Self, ()>
    where
        Self: Sized,
    {
        match s.to_ascii_lowercase().as_str() {
            "0" | "f" | "false" => Ok(false.into()),
            "1" | "t" | "true" => Ok(true.into()),
            _ => Err(()),
        }
    }
}
