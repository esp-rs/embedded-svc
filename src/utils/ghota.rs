use core::cmp::max;

use serde::{Deserialize, Serialize};

use crate::errors::Errors;
use crate::http::{client::*, Headers};
use crate::io;
use crate::ota::*;

// Copied from here:
// https://github.com/XAMPPRocky/octocrab/blob/master/src/models/repos.rs
// To conserve memory, unly the utilized fields are mapped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Release<'a, const N: usize = 32> {
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
pub struct Asset<'a> {
    pub browser_download_url: &'a str,
    pub name: &'a str,
    pub label: Option<&'a str>,
    // pub state: String,
    // pub content_type: String,
    // pub size: i64,
    pub updated_at: &'a str,
    // pub created_at: DateTime<Utc>,
}

impl<'a> From<(&Release<'a>, &Asset<'a>)> for FirmwareInfo<'a> {
    fn from((release, asset): (&Release<'a>, &Asset<'a>)) -> Self {
        Self {
            version: release.tag_name,
            released: asset.updated_at,
            description: release.body.unwrap_or_else(|| "".into()),
            signature: None,
            download_id: Some(asset.browser_download_url),
        }
    }
}

pub struct GitHubOtaService<'a, C> {
    base_url: &'a str,
    label: &'a str,
    client: C,
    buf: &'a mut [u8],
}

impl<'a, C> GitHubOtaService<'a, C>
where
    C: Client,
{
    pub fn new(base_url: &'a str, label: &'a str, client: C, buf: &'a mut [u8]) -> Self {
        Self {
            base_url,
            label,
            client,
            buf,
        }
    }

    // pub fn new_with_repo(repo: &str, project: &str, label: &'a str, client: C) -> Self {
    //     Self::new(
    //         join(join("https://api.github.com/repos", repo), project),
    //         label,
    //         client,
    //     )
    // }

    fn get_gh_releases<const N: usize>(
        &mut self,
    ) -> Result<heapless::Vec<Release<'_>, N>, C::Error> {
        let response = self
            .client
            .get(join::<256>(self.base_url, "releases"))?
            .submit()?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, self.buf)?;

        let releases = serde_json::from_slice::<heapless::Vec<Release<'_>, N>>(buf).unwrap();

        Ok(releases)
    }

    fn fill_gh_releases<const N: usize>(
        &'a mut self,
        releases: &'a mut [Release<'a>],
    ) -> Result<(&'a [Release<'a>], usize), C::Error>
    where
        Self: 'a,
    {
        let response = self
            .client
            .get(&join::<128>(self.base_url, "releases"))?
            .submit()?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, self.buf)?;

        let releases_vec = serde_json::from_slice::<heapless::Vec<Release<'a>, N>>(buf).unwrap();

        let cnt = max(releases.len(), releases_vec.len());
        releases[..cnt].clone_from_slice(&releases_vec[..cnt]);

        Ok((&releases[..cnt], cnt))
    }

    fn get_gh_latest_release(&mut self) -> Result<Option<Release<'_>>, C::Error> {
        let response = self
            .client
            .get(&join::<128>(
                &join::<128>(self.base_url, "release"),
                "latest",
            ))?
            .submit()?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, self.buf)?;

        let release = serde_json::from_slice::<Option<Release<'_>>>(buf).unwrap();

        Ok(release)
    }

    fn get_firmware<'b, const N: usize, I>(
        label: &'b str,
        releases: I,
    ) -> impl Iterator<Item = FirmwareInfo<'b>> + 'b
    where
        I: Iterator<Item = Release<'b>> + 'b,
    {
        releases.flat_map(move |release| {
            release
                .assets
                .iter()
                .filter(|asset| asset.label.as_ref().map(|l| *l == label).unwrap_or(false))
                .map(|asset| FirmwareInfo::from((&release, asset)))
                .collect::<heapless::Vec<_, N>>()
        })
    }
}

pub struct GitHubOtaRead<R> {
    size: Option<usize>,
    response: R,
}

impl<S> Errors for GitHubOtaRead<S>
where
    S: Errors,
{
    type Error = S::Error;
}

impl<R> OtaRead for GitHubOtaRead<R>
where
    R: Response,
{
    fn size(&self) -> Option<usize> {
        self.size
    }
}

impl<R> io::Read for GitHubOtaRead<R>
where
    R: Response,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.response.reader().read(buf)
    }
}

impl<'a, C> Errors for GitHubOtaService<'a, C>
where
    C: Errors,
{
    type Error = C::Error;
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

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo<'_>>, Self::Error> {
        let label = self.label;

        let release = self.get_gh_latest_release().unwrap().unwrap();
        let releases = core::iter::once(release);

        let firmware: Option<FirmwareInfo<'_>> =
            Self::get_firmware::<32, _>(label, releases).next();

        Ok(firmware)
    }

    #[cfg(not(feature = "alloc"))]
    fn fill_releases(
        &mut self,
        infos: &mut [FirmwareInfo<'_>],
    ) -> Result<(&[FirmwareInfo<'_>], usize), Self::Error> {
    }

    #[cfg(feature = "alloc")]
    fn get_releases(&mut self) -> Result<alloc::vec::Vec<FirmwareInfo<'_>>, Self::Error> {
        let label = self.label;

        let assets =
            Self::get_firmware::<32, _>(label, self.get_gh_releases::<64>()?.into_iter()).collect();

        Ok(assets)
    }

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error> {
        let response = self.client.get(download_id)?.submit()?;

        Ok(GitHubOtaRead {
            size: response.content_len(),
            response,
        })
    }
}

fn join<const N: usize>(uri: &str, path: &str) -> heapless::String<N> {
    let uri_slash = uri.ends_with('/');
    let path_slash = path.starts_with('/');

    if path.is_empty() || path.len() == 1 && uri_slash && path_slash {
        uri.into()
    } else {
        let path = if uri_slash && path_slash {
            &path[1..]
        } else {
            path
        };

        let mut result = heapless::String::from(uri);

        if !uri_slash && !path_slash {
            result.push('/').unwrap();
        }

        result.push_str(path).unwrap();

        result
    }
}
