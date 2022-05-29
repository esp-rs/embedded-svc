use core::mem;

extern crate alloc;
use alloc::borrow::{Cow, ToOwned};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::errors::Errors;
use crate::{
    http::{client::*, Headers},
    io,
    ota::*,
};

// Copied from here:
// https://github.com/XAMPPRocky/octocrab/blob/master/src/models/repos.rs
// To conserve memory, unly the utilized fields are mapped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Release<'a> {
    pub tag_name: &'a str,
    pub body: Option<&'a str>,
    pub draft: bool,
    pub prerelease: bool,
    // pub created_at: Option<DateTime<Utc>>,
    // pub published_at: Option<DateTime<Utc>>,
    pub assets: Vec<Asset<'a>>,
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

impl<'a> From<(Release<'a>, Asset<'a>)> for FirmwareInfo<'a> {
    fn from((release, asset): (Release, Asset<'a>)) -> Self {
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
    base_url: Cow<'a, str>,
    label: Cow<'a, str>,
    client: C,
}

impl<'a, C> GitHubOtaService<'a, C>
where
    C: Client,
{
    pub fn new(
        base_url: impl Into<Cow<'a, str>>,
        label: impl Into<Cow<'a, str>>,
        client: C,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            label: label.into(),
            client,
        }
    }

    pub fn new_with_repo(
        repo: impl AsRef<str>,
        project: impl AsRef<str>,
        label: impl Into<Cow<'a, str>>,
        client: C,
    ) -> Self {
        Self::new(
            join(join("https://api.github.com/repos", repo), project),
            label,
            client,
        )
    }

    fn get_gh_releases(&mut self) -> Result<impl Iterator<Item = Release>, C::Error> {
        let response = self
            .client
            .get(join(self.base_url.as_ref(), "releases"))?
            .submit()?;

        let mut read = response.reader();

        // TODO: Deserialization code below is not efficient
        // See this for a common workaround: https://github.com/serde-rs/json/issues/404#issuecomment-892957228

        // TODO: Need to implement our own error type
        #[cfg(feature = "std")]
        let releases = serde_json::from_reader::<_, Vec<Release>>(io::adapters::ToStd::new(&mut read)).unwrap();

        #[cfg(not(feature = "std"))]
        let releases = {
            let body: Result<Vec<u8>, _> = io::Bytes::<_, 64>::new(&mut read).collect();

            let bytes = body?;

            serde_json::from_slice::<Vec<Release>>(&bytes).unwrap()
        };

        Ok(releases.into_iter())
    }

    fn get_gh_latest_release(&mut self) -> Result<Option<Release>, C::Error> {
        let response = self
            .client
            .get(join(join(self.base_url.as_ref(), "release"), "latest"))?
            .submit()?;

        let mut read = response.reader();

        // TODO: Need to implement our own error type
        #[cfg(feature = "std")]
        let release =
            serde_json::from_reader::<_, Option<Release>>(io::adapters::ToStd::new(&mut read)).unwrap();

        #[cfg(not(feature = "std"))]
        let release = {
            let body: Result<Vec<u8>, _> = io::Bytes::<_, 64>::new(&mut read).collect();

            let bytes = body?;

            serde_json::from_slice::<Option<Release>>(&bytes).unwrap()
        };

        Ok(release)
    }

    fn get_gh_assets(
        &self,
        releases: impl Iterator<Item = Release<'a>>,
    ) -> impl Iterator<Item = (Release<'a>, Asset<'a>)> {
        let label = self.label.as_ref().to_owned();

        releases
            .flat_map(|mut release| {
                let mut assets = Vec::new();
                mem::swap(&mut release.assets, &mut assets);

                assets
                    .into_iter()
                    .map(move |asset| (release.clone(), asset))
            })
            .filter(move |(_release, asset)| {
                asset
                    .label
                    .as_ref()
                    .map(|l| l == label.as_str())
                    .unwrap_or(false)
            })
    }
}

pub struct OtaServerIterator(Box<dyn Iterator<Item = FirmwareInfo>>);

impl Iterator for OtaServerIterator {
    type Item = FirmwareInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
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

    type Iterator = OtaServerIterator;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error> {
        // TODO: Need to implement our own error type
        let releases = self.get_gh_latest_release().unwrap().into_iter();
        Ok(self.get_gh_assets(releases).map(Into::into).next())
    }

    fn get_releases(&mut self) -> Result<Self::Iterator, Self::Error> {
        let releases = self.get_gh_releases()?;
        let assets = self.get_gh_assets(releases).map(Into::into);

        Ok(OtaServerIterator(Box::new(assets)))
    }

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error> {
        let response = self.client.get(download_id)?.submit()?;

        Ok(GitHubOtaRead {
            size: response.content_len(),
            response,
        })
    }
}

fn join<'a>(uri: impl Into<Cow<'a, str>>, path: impl AsRef<str>) -> Cow<'a, str> {
    let uri = uri.into();
    let path = path.as_ref();

    let uri_slash = uri.ends_with('/');
    let path_slash = path.starts_with('/');

    if path.is_empty() || path.len() == 1 && uri_slash && path_slash {
        uri
    } else {
        let path = if uri_slash && path_slash {
            &path[1..]
        } else {
            path
        };

        let mut result = uri.into_owned();

        if !uri_slash && !path_slash {
            result.push('/');
        }

        result.push_str(path);

        Cow::Owned(result)
    }
}
