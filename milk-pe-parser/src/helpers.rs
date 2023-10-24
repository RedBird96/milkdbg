pub fn parse<T>(s: &[u8]) -> nom::IResult<&[u8], &T> {
    let count = std::mem::size_of::<T>();
    let (s, bytes) = nom::bytes::complete::take(count)(s)?;
    let value = unsafe { std::mem::transmute(bytes.as_ptr()) };
    Ok((s, value))
}

pub fn parse_slice<T>(s: &[u8], len: usize) -> nom::IResult<&[u8], &[T]> {
    let total_size = std::mem::size_of::<T>() * len;
    let (s, bytes) = nom::bytes::complete::take(total_size)(s)?;
    let value = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const T, len) };
    Ok((s, value))
}

pub fn take(s: &[u8], count: usize) -> nom::IResult<&[u8], &[u8]> {
    nom::bytes::complete::take(count)(s)
}

pub fn take_untill_value(s: &[u8], value: u8) -> nom::IResult<&[u8], &[u8]> {
    let (s, v) = nom::bytes::complete::take_till(|x| x == value)(s)?;
    Ok((s, v))
}
