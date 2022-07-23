use core::str;

use uncased::UncasedStr;

pub mod server;

#[derive(Debug)]
pub struct Headers<'b, const N: usize>([(&'b str, &'b str); N]);

impl<'b, const N: usize> Headers<'b, N> {
    pub const fn new() -> Self {
        Self([("", ""); N])
    }

    pub fn content_len(&self) -> Option<u64> {
        self.get("Content-Length")
            .map(|content_len_str| content_len_str.parse::<u64>().unwrap())
    }

    pub fn content_type(&self) -> Option<&str> {
        self.get("Content-Type")
    }

    pub fn content_encoding(&self) -> Option<&str> {
        self.get("Content-Encoding")
    }

    pub fn transfer_encoding(&self) -> Option<&str> {
        self.get("Transfer-Encoding")
    }

    pub fn connection(&self) -> Option<&str> {
        self.get("Connection")
    }

    pub fn cache_control(&self) -> Option<&'_ str> {
        self.get("Cache-Control")
    }

    pub fn upgrade(&self) -> Option<&'_ str> {
        self.get("Upgrade")
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .filter(|header| !header.0.is_empty())
            .map(|header| (header.0, header.1))
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.iter()
            .find(|(hname, _)| UncasedStr::new(name) == UncasedStr::new(hname))
            .map(|(_, value)| value)
    }

    pub fn set(&mut self, name: &'b str, value: &'b str) -> &mut Self {
        for header in &mut self.0 {
            if header.0.is_empty() || UncasedStr::new(header.0) == UncasedStr::new(name) {
                *header = (name, value);
                return self;
            }
        }

        panic!("No space left");
    }

    pub fn remove(&mut self, name: &str) -> &mut Self {
        let index = self
            .0
            .iter()
            .enumerate()
            .find(|(_, header)| UncasedStr::new(header.0) == UncasedStr::new(name));

        if let Some((mut index, _)) = index {
            while index < self.0.len() - 1 {
                self.0[index] = self.0[index + 1];

                index += 1;
            }

            self.0[index] = ("", "");
        }

        self
    }

    pub fn set_content_len(
        &mut self,
        content_len: u64,
        buf: &'b mut heapless::String<20>,
    ) -> &mut Self {
        *buf = heapless::String::<20>::from(content_len);

        self.set("Content-Length", buf.as_str())
    }

    pub fn set_content_type(&mut self, content_type: &'b str) -> &mut Self {
        self.set("Content-Type", content_type)
    }

    pub fn set_content_encoding(&mut self, content_encoding: &'b str) -> &mut Self {
        self.set("Content-Encoding", content_encoding)
    }

    pub fn set_transfer_encoding(&mut self, transfer_encoding: &'b str) -> &mut Self {
        self.set("Transfer-Encoding", transfer_encoding)
    }

    pub fn set_transfer_encoding_chunked(&mut self) -> &mut Self {
        self.set_transfer_encoding("Chunked")
    }

    pub fn set_connection(&mut self, connection: &'b str) -> &mut Self {
        self.set("Connection", connection)
    }

    pub fn set_connection_close(&mut self) -> &mut Self {
        self.set_connection("Close")
    }

    pub fn set_connection_keep_alive(&mut self) -> &mut Self {
        self.set_connection("Keep-Alive")
    }

    pub fn set_connection_upgrade(&mut self) -> &mut Self {
        self.set_connection("Upgrade")
    }

    pub fn set_cache_control(&mut self, cache: &'b str) -> &mut Self {
        self.set("Cache-Control", cache)
    }

    pub fn set_cache_control_no_cache(&mut self) -> &mut Self {
        self.set_cache_control("No-Cache")
    }

    pub fn set_upgrade(&mut self, upgrade: &'b str) -> &mut Self {
        self.set("Upgrade", upgrade)
    }

    pub fn set_upgrade_websocket(&mut self) -> &mut Self {
        self.set_upgrade("websocket")
    }

    pub fn as_slice(&self) -> &[(&'b str, &'b str)] {
        let index = self
            .0
            .iter()
            .enumerate()
            .find(|(_, header)| header.0.is_empty())
            .map(|(index, _)| index)
            .unwrap_or(N);

        &self.0[..index]
    }

    pub fn release(self) -> [(&'b str, &'b str); N] {
        self.0
    }
}

impl<'b, const N: usize> Default for Headers<'b, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'b, const N: usize> crate::http::Headers for Headers<'b, N> {
    fn header(&self, name: &str) -> Option<&'_ str> {
        self.get(name)
    }
}
