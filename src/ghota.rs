use core::mem;

extern crate alloc;
use alloc::borrow::Cow;

use anyhow;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::{
    http::{client::*, HttpHeaders},
    io::{self, StdIO},
    ota::*,
};

// Copied from here:
// https://github.com/XAMPPRocky/octocrab/blob/master/src/models/repos.rs
// To conserve memory, unly the utilized fields are mapped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Release {
    pub tag_name: String,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    // pub created_at: Option<DateTime<Utc>>,
    // pub published_at: Option<DateTime<Utc>>,
    pub assets: Vec<Asset>,
}

// Copied from here:
// https://github.com/XAMPPRocky/octocrab/blob/master/src/models/repos.rs
// To conserve memory, unly the utilized fields are mapped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Asset {
    pub browser_download_url: String,
    pub name: String,
    pub label: Option<String>,
    // pub state: String,
    // pub content_type: String,
    // pub size: i64,
    pub updated_at: String,
    // pub created_at: DateTime<Utc>,
}

impl From<(Release, Asset)> for FirmwareInfo {
    fn from((release, asset): (Release, Asset)) -> Self {
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
    C: HttpClient,
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

    fn get_gh_releases(&mut self) -> Result<impl Iterator<Item = Release>, anyhow::Error> {
        let response = self
            .client
            .get(join(self.base_url.as_ref(), "releases"))?
            .submit()?;

        // TODO: Deserialization code below is not efficient
        // See this for a common workaround: https://github.com/serde-rs/json/issues/404#issuecomment-892957228

        #[cfg(feature = "std")]
        let releases =
            serde_json::from_reader::<_, Vec<Release>>(StdIO(&mut response.into_payload()))?;

        #[cfg(not(feature = "std"))]
        let releases = serde_json::from_slice::<_, Vec<Release>>(
            &StdIO(&mut response.into_payload()).read_to_end(),
        )?;

        Ok(releases.into_iter())
    }

    fn get_gh_latest_release(&mut self) -> Result<Option<Release>, anyhow::Error> {
        let response = self
            .client
            .get(join(join(self.base_url.as_ref(), "release"), "latest"))?
            .submit()?;

        let release =
            serde_json::from_reader::<_, Option<Release>>(StdIO(&mut response.into_payload()))?;

        Ok(release)
    }

    fn get_gh_assets(
        &self,
        releases: impl Iterator<Item = Release>,
    ) -> impl Iterator<Item = (Release, Asset)> {
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
    read: R,
}

impl<R, E> OtaRead for GitHubOtaRead<R>
where
    R: io::Read<Error = E>,
    E: Into<anyhow::Error>,
{
    fn size(&self) -> Option<usize> {
        self.size
    }
}

impl<R, E> io::Read for GitHubOtaRead<R>
where
    R: io::Read<Error = E>,
    E: Into<anyhow::Error>,
{
    type Error = anyhow::Error;

    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read.do_read(buf).map_err(Into::into)
    }
}

impl<'a, C> OtaServer for GitHubOtaService<'a, C>
where
    C: HttpClient + 'static,
{
    type Error = anyhow::Error;

    type OtaRead<'b> =
        GitHubOtaRead<
            <<<C as HttpClient>::Request<'b> as HttpRequest<'b>>::Response<'b> as HttpResponse<
                'b,
            >>::Read<'b>,
        >;

    type Iterator = OtaServerIterator;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error> {
        let releases = self.get_gh_latest_release()?.into_iter();
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
            read: response.into_payload(),
        })
    }
}

fn join<'a>(uri: impl Into<Cow<'a, str>>, path: impl AsRef<str>) -> Cow<'a, str> {
    let uri = uri.into();
    let path = path.as_ref();

    let uri_slash = uri.ends_with("/");
    let path_slash = path.starts_with("/");

    if path.len() == 0 || path.len() == 1 && uri_slash && path_slash {
        uri
    } else {
        let path = if uri_slash && path_slash {
            &path[1..]
        } else {
            path
        };

        let mut result = uri.into_owned();

        if !uri_slash && !path_slash {
            result.push_str("/");
        }

        result.push_str(path);

        Cow::Owned(result)
    }
}
