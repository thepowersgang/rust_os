pub struct PcapWriter<W>
{
    inner: W,
    start_time_unix_s: u32,
    start_time_i: ::std::time::Instant,
}

#[derive(::serde_derive::Serialize)]
struct FileHeader
{
    magic_number: u32,
    version_major: u16,
    version_minor: u16,
    thiszone: i32,
    sigfigs: u32,
    snaplen: u32,
    network: i32,
}
#[derive(::serde_derive::Serialize)]
struct PacketHeader
{
    timestamp_utc_s: u32,
    timestamp_utc_us: u32,
    captured_length: u32,
    original_length: u32,
}

fn bc_ser<W: ::std::io::Write, T: ::serde::Serialize>(dst: W, v: &T) -> Result<(),::bincode::Error> {
    use ::bincode::Options;
    ::bincode::options().allow_trailing_bytes().with_fixint_encoding()
        .serialize_into(dst, v)
}

impl<W: ::std::io::Write> PcapWriter<W>
{
    pub fn new(mut inner: W) -> Result<Self,::bincode::Error> {
        bc_ser(&mut inner, &FileHeader
            {
            magic_number: 0xa1b2c3d4_u32,   // Indicates microsecond accuracy (as opposed to nanoseconds)
            version_major: 2,
            version_minor: 4,
            thiszone: 0,    // Timezone correction (Seconds)
            sigfigs: 0, // Mostly ignored
            snaplen: 1560,  // Maximum packet length
            network: 1 /*LINKTYPE_ETHERNET*/,
            })?;
        Ok(PcapWriter {
            inner,
            start_time_unix_s: ::std::time::SystemTime::now().duration_since(::std::time::UNIX_EPOCH).unwrap().as_secs() as u32,
            start_time_i: ::std::time::Instant::now(),
        })
    }

    pub fn push_packet(&mut self, data: &[u8]) -> Result<(),::bincode::Error> {
        let dt = ::std::time::Instant::now() - self.start_time_i;
        bc_ser(&mut self.inner, &PacketHeader
            {
            timestamp_utc_s: dt.as_secs() as u32 + self.start_time_unix_s,
            timestamp_utc_us: dt.subsec_micros(),
            original_length: data.len() as u32,
            captured_length: data.len() as u32,
            })?;
        self.inner.write(data)?;
        Ok( () )
    }
}