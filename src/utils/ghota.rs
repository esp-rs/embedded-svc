use core::convert::TryInto;
use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::http::{client::*, Headers};
use crate::io::{self, ErrorKind, Io, Read};
use crate::ota::*;

#[derive(Debug)]
pub enum Error<E> {
    UrlOverflow,
    BufferOverflow,
    FirmwareInfoOverflow,
    Http(E),
}

impl<E> io::Error for Error<E>
where
    E: io::Error,
{
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

// Copied from here:
// https://github.com/XAMPPRocky/octocrab/blob/master/src/models/repos.rs
// To conserve memory, unly the utilized fields are mapped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Release<'a, const N: usize = 32> {
    pub tag_name: &'a str,
    pub body: Option<&'a str>,
    pub draft: bool,
    pub prerelease: bool,
    // pub created_at: Option<DateTime<Utc>>,
    // pub published_at: Option<DateTime<Utc>>,
    pub assets: heapless::Vec<Asset<'a>, N>,
}

// Copied from here:
// https://github.com/XAMPPRocky/octocrab/blob/master/src/models/repos.rs
// To conserve memory, unly the utilized fields are mapped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Asset<'a> {
    pub browser_download_url: &'a str,
    pub name: &'a str,
    pub label: Option<&'a str>,
    // pub state: String,
    // pub content_type: String,
    // pub size: i64,
    pub updated_at: &'a str,
    // pub created_at: DateTime<Utc>,
}

impl<'a> Asset<'a> {
    fn as_firmware_info<E>(&'a self, release: &'a Release<'a>) -> Result<FirmwareInfo, Error<E>>
    where
        E: io::Error,
    {
        Ok(FirmwareInfo {
            version: release
                .tag_name
                .try_into()
                .map_err(|_| Error::FirmwareInfoOverflow)?,
            released: self
                .updated_at
                .try_into()
                .map_err(|_| Error::FirmwareInfoOverflow)?,
            description: if let Some(body) = release.body {
                Some(body.try_into().map_err(|_| Error::FirmwareInfoOverflow)?)
            } else {
                None
            },
            signature: None,
            download_id: Some(
                self.browser_download_url
                    .try_into()
                    .map_err(|_| Error::FirmwareInfoOverflow)?,
            ),
        })
    }
}

pub struct GitHubOtaService<'a, C, const B: usize = 1024, const U: usize = 256> {
    base_url: heapless::String<U>,
    label: &'a str,
    client: C,
    buf: [u8; B],
}

impl<'a, C, const B: usize, const U: usize> GitHubOtaService<'a, C, B, U>
where
    C: Client,
{
    pub fn new(base_url: &str, label: &'a str, client: C) -> Result<Self, Error<C::Error>> {
        Ok(Self {
            base_url: base_url.try_into().map_err(|_| Error::UrlOverflow)?,
            label,
            client,
            buf: [0_u8; B],
        })
    }

    pub fn new_with_repo(
        repo: &str,
        project: &str,
        label: &'a str,
        client: C,
    ) -> Result<Self, Error<C::Error>> {
        Self::new(
            &join::<U, _>(
                &join::<U, _>("https://api.github.com/repos", repo)?,
                project,
            )?,
            label,
            client,
        )
    }

    fn get_gh_releases_n<const N: usize>(
        &mut self,
    ) -> Result<(heapless::Vec<Release<'_>, N>, &str), Error<C::Error>> {
        let mut response = self
            .client
            .get(join::<U, _>(&self.base_url, "releases")?)
            .map_err(Error::Http)?
            .submit()
            .map_err(Error::Http)?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, &mut self.buf).map_err(Error::Http)?;

        let releases = serde_json::from_slice::<heapless::Vec<Release<'_>, N>>(buf).unwrap(); // TODO

        Ok((releases, self.label))
    }

    #[cfg(feature = "alloc")]
    fn get_gh_releases(&mut self) -> Result<(alloc::vec::Vec<Release<'_>>, &str), Error<C::Error>> {
        let mut response = self
            .client
            .get(join::<U, _>(&self.base_url, "releases")?)
            .map_err(Error::Http)?
            .submit()
            .map_err(Error::Http)?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, &mut self.buf).map_err(Error::Http)?;

        let releases = serde_json::from_slice::<alloc::vec::Vec<Release<'_>>>(buf).unwrap(); // TODO

        Ok((releases, self.label))
    }

    fn get_gh_latest_release(&mut self) -> Result<Option<Release<'_>>, Error<C::Error>> {
        let mut response = self
            .client
            .get(&join::<U, _>(
                &join::<U, _>(&self.base_url, "release")?,
                "latest",
            )?)
            .map_err(Error::Http)?
            .submit()
            .map_err(Error::Http)?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, &mut self.buf).map_err(Error::Http)?;

        let release = serde_json::from_slice::<Option<Release<'_>>>(buf).unwrap(); // TODO

        Ok(release)
    }
}

pub struct GitHubOtaRead<R> {
    size: Option<usize>,
    response: R,
}

impl<S> Io for GitHubOtaRead<S>
where
    S: Response,
{
    type Error = Error<S::Error>;
}

impl<R> OtaRead for GitHubOtaRead<R>
where
    R: Response,
{
    fn size(&self) -> Option<usize> {
        self.size
    }
}

impl<R> Read for GitHubOtaRead<R>
where
    R: Response,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.response.reader().read(buf).map_err(Error::Http)
    }
}

impl<'a, C> Io for GitHubOtaService<'a, C>
where
    C: Io,
{
    type Error = Error<C::Error>;
}

impl<'a, C> OtaServer for GitHubOtaService<'a, C>
where
    C: Client + 'static,
{
    type OtaRead<'b>
    where
        Self: 'b,
    = GitHubOtaRead<
        <<<C as Client>::Request<'b> as Request<'b>>::Write<'b> as RequestWrite<'b>>::Response,
    >;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error> {
        let label = self.label;

        let release = self.get_gh_latest_release()?;

        if let Some(release) = release.as_ref() {
            for asset in &release.assets {
                if asset.label == Some(label) {
                    return Ok(Some(asset.as_firmware_info(release)?));
                }
            }
        }

        Ok(None)
    }

    #[cfg(feature = "alloc")]
    fn get_releases(&mut self) -> Result<alloc::vec::Vec<FirmwareInfo>, Self::Error> {
        let (releases, label) = self.get_gh_releases()?;

        Ok(releases
            .iter()
            .flat_map(|release| {
                release
                    .assets
                    .iter()
                    .filter(|asset| asset.label.as_ref().map(|l| *l == label).unwrap_or(false))
                    .map(move |asset| asset.as_firmware_info(release))
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn get_releases_n<const N: usize>(
        &mut self,
    ) -> Result<heapless::Vec<FirmwareInfo, N>, Self::Error> {
        let (releases, label) = self.get_gh_releases_n::<N>()?;

        Ok(releases
            .iter()
            .flat_map(|release| {
                release
                    .assets
                    .iter()
                    .filter(|asset| asset.label.as_ref().map(|l| *l == label).unwrap_or(false))
                    .map(move |asset| asset.as_firmware_info(release))
            })
            .collect::<Result<heapless::Vec<_, N>, _>>()?)
    }

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error> {
        let response = self
            .client
            .get(download_id)
            .map_err(Error::Http)?
            .submit()
            .map_err(Error::Http)?;

        Ok(GitHubOtaRead {
            size: response.content_len(),
            response,
        })
    }
}

fn join<const N: usize, E>(uri: &str, path: &str) -> Result<heapless::String<N>, Error<E>>
where
    E: io::Error,
{
    let uri_slash = uri.ends_with('/');
    let path_slash = path.starts_with('/');

    let uri = if path.is_empty() || path.len() == 1 && uri_slash && path_slash {
        uri.into()
    } else {
        let path = if uri_slash && path_slash {
            &path[1..]
        } else {
            path
        };

        let mut result = heapless::String::from(uri);

        if !uri_slash && !path_slash {
            result.push('/').map_err(|_| Error::UrlOverflow)?;
        }

        result.push_str(path).map_err(|_| Error::UrlOverflow)?;

        result
    };

    Ok(uri)
}
