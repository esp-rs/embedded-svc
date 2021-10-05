use core::mem;

extern crate alloc;
use alloc::borrow::Cow;

use url;

use anyhow;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::{
    http::client::*,
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
    repo: Cow<'a, str>,
    project: Cow<'a, str>,
    label: Cow<'a, str>,
    client: C,
}

impl<'a, C> GitHubOtaService<'a, C>
where
    C: HttpClient,
{
    pub fn new(
        repo: impl Into<Cow<'a, str>>,
        project: impl Into<Cow<'a, str>>,
        label: impl Into<Cow<'a, str>>,
        client: C,
    ) -> Self {
        Self {
            repo: repo.into(),
            project: project.into(),
            label: label.into(),
            client,
        }
    }

    fn get_gh_releases(&mut self) -> Result<impl Iterator<Item = Release>, anyhow::Error> {
        let response = self
            .client
            .get(self.get_base_url()?.join("releases")?)?
            .submit()?;

        // TODO: Not efficient
        let releases =
            serde_json::from_reader::<_, Vec<Release>>(StdIO(&mut response.into_payload()))?;

        Ok(releases.into_iter())
    }

    fn get_gh_latest_release(&mut self) -> Result<Option<Release>, anyhow::Error> {
        let response = self
            .client
            .get(self.get_base_url()?.join("release")?.join("latest")?)?
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

    fn get_base_url(&self) -> Result<url::Url, anyhow::Error> {
        Ok(url::Url::parse("https://api.github.com/repos")?
            .join(self.repo.as_ref())?
            .join(self.project.as_ref())?)
    }
}

pub struct OtaServerIterator(Box<dyn Iterator<Item = FirmwareInfo>>);

impl Iterator for OtaServerIterator {
    type Item = FirmwareInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct OtaRead<R>(R);

impl<R, E> io::Read for OtaRead<R>
where
    R: io::Read<Error = E>,
    E: std::error::Error + Send + Sync + 'static,
{
    type Error = anyhow::Error;

    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(self.0.do_read(buf)?)
    }
}

impl<'a, C> OtaServer for GitHubOtaService<'a, C>
where
    C: HttpClient + 'static,
{
    type Error = anyhow::Error;

    type Read<'b> =
        OtaRead<
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

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::Read<'_>, Self::Error> {
        Ok(OtaRead(
            self.client
                .get(url::Url::parse(download_id.as_ref())?)?
                .submit()?
                .into_payload(),
        ))
    }
}
