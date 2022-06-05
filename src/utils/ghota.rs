use core::convert::TryFrom;
use core::mem::{self, MaybeUninit};

use serde::{Deserialize, Serialize};

use crate::errors::{EitherError, Errors};
use crate::http::{client::*, Headers};
use crate::io;
use crate::ota::*;
use crate::strconv::StrConvError;

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

impl<'a, S> TryFrom<(&Release<'a>, &Asset<'a>)> for FirmwareInfo<S>
where
    S: TryFrom<&'a str>,
{
    type Error = StrConvError;

    fn try_from((release, asset): (&Release<'a>, &Asset<'a>)) -> Result<Self, Self::Error> {
        Ok(Self {
            version: S::try_from(release.tag_name).map_err(|_| StrConvError)?,
            released: S::try_from(asset.updated_at).map_err(|_| StrConvError)?,
            description: S::try_from(release.body.unwrap_or_else(|| ""))
                .map_err(|_| StrConvError)?,
            signature: None,
            download_id: Some(S::try_from(asset.browser_download_url).map_err(|_| StrConvError)?),
        })
    }
}

pub struct GitHubOtaService<'a, C, const N: usize = 128, const U: usize = 256> {
    base_url: &'a str,
    label: &'a str,
    client: C,
    buf: &'a mut [u8],
}

impl<'a, C, const N: usize, const U: usize> GitHubOtaService<'a, C, N, U>
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

    fn get_gh_releases(
        &mut self,
    ) -> Result<(heapless::Vec<Release<'_>, N>, &str), EitherError<C::Error, StrConvError>> {
        let response = self
            .client
            .get(join::<U>(self.base_url, "releases").map_err(EitherError::Second)?)
            .map_err(EitherError::First)?
            .submit()
            .map_err(EitherError::First)?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, self.buf).map_err(EitherError::First)?;

        let releases = serde_json::from_slice::<heapless::Vec<Release<'_>, N>>(buf).unwrap(); // TODO

        Ok((releases, self.label))
    }

    // fn fill_gh_releases<const N: usize>(
    //     &'a mut self,
    //     releases: &'a mut [Release<'a>],
    // ) -> Result<(&'a [Release<'a>], usize), C::Error>
    // where
    //     Self: 'a,
    // {
    //     let response = self
    //         .client
    //         .get(&join::<128>(self.base_url, "releases"))?
    //         .submit()?;

    //     let mut read = response.reader();

    //     let (buf, _) = io::read_max(&mut read, self.buf)?;

    //     let releases_vec = serde_json::from_slice::<heapless::Vec<Release<'a>, N>>(buf).unwrap();

    //     let cnt = max(releases.len(), releases_vec.len());
    //     releases[..cnt].clone_from_slice(&releases_vec[..cnt]);

    //     Ok((&releases[..cnt], cnt))
    // }

    fn get_gh_latest_release(
        &mut self,
    ) -> Result<Option<Release<'_>>, EitherError<C::Error, StrConvError>> {
        let response = self
            .client
            .get(
                &join::<U>(
                    &join::<U>(self.base_url, "release").map_err(EitherError::Second)?,
                    "latest",
                )
                .map_err(EitherError::Second)?,
            )
            .map_err(EitherError::First)?
            .submit()
            .map_err(EitherError::First)?;

        let mut read = response.reader();

        let (buf, _) = io::read_max(&mut read, self.buf).map_err(EitherError::First)?;

        let release = serde_json::from_slice::<Option<Release<'_>>>(buf).unwrap(); // TODO

        Ok(release)
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

    fn get_latest_release<'b, S>(
        &'b mut self,
    ) -> Result<Option<FirmwareInfo<S>>, EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'b str>,
    {
        let label = self.label;

        let release = self.get_gh_latest_release()?;

        if let Some(release) = release.as_ref() {
            for asset in &release.assets {
                if asset.label == Some(label) {
                    return Ok(Some(
                        FirmwareInfo::try_from((release, asset)).map_err(EitherError::Second)?,
                    ));
                }
            }
        }

        Ok(None)
    }

    fn fill_releases<'b, 'c, S>(
        &'b mut self,
        infos: &'c mut [MaybeUninit<FirmwareInfo<S>>],
    ) -> Result<(&'c [FirmwareInfo<S>], usize), EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'b str>,
    {
        let (releases, label) = self.get_gh_releases()?;

        let iter = releases.iter().flat_map(|release| {
            release
                .assets
                .iter()
                .filter(|asset| asset.label.as_ref().map(|l| *l == label).unwrap_or(false))
                .map(move |asset| FirmwareInfo::try_from((release, asset)))
        });

        let mut len = 0_usize;
        let mut max_len = 0_usize;
        for (index, info) in iter.enumerate() {
            let info = info.map_err(EitherError::Second)?;

            max_len = index + 1;

            if index < infos.len() {
                len = index + 1;
                infos[index].write(info);
            }
        }

        Ok((unsafe { mem::transmute(&infos[0..len]) }, max_len))
    }

    #[cfg(feature = "alloc")]
    fn get_releases(
        &mut self,
    ) -> Result<alloc::vec::Vec<FirmwareInfo<alloc::string::String>>, Self::Error> {
        let (releases, label) = self.get_gh_releases().map_err(|e| match e {
            EitherError::First(e) => e,
            EitherError::Second(_) => unreachable!(),
        })?;

        Ok(releases
            .iter()
            .flat_map(|release| {
                release
                    .assets
                    .iter()
                    .filter(|asset| asset.label.as_ref().map(|l| *l == label).unwrap_or(false))
                    .map(move |asset| FirmwareInfo::try_from((release, asset)))
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap())
    }

    #[cfg(feature = "heapless")]
    fn get_releases_heapless<'b, S, const N: usize>(
        &'b mut self,
    ) -> Result<heapless::Vec<FirmwareInfo<S>, N>, EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'b str>,
    {
        let (releases, label) = self.get_gh_releases()?;

        Ok(releases
            .iter()
            .flat_map(|release| {
                release
                    .assets
                    .iter()
                    .filter(|asset| asset.label.as_ref().map(|l| *l == label).unwrap_or(false))
                    .map(move |asset| FirmwareInfo::try_from((release, asset)))
            })
            .collect::<Result<heapless::Vec<_, N>, _>>()
            .map_err(EitherError::Second)?)
    }

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error> {
        let response = self.client.get(download_id)?.submit()?;

        Ok(GitHubOtaRead {
            size: response.content_len(),
            response,
        })
    }
}

fn join<const N: usize>(uri: &str, path: &str) -> Result<heapless::String<N>, StrConvError> {
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
            result.push('/').map_err(|_| StrConvError)?;
        }

        result.push_str(path).map_err(|_| StrConvError)?;

        result
    };

    Ok(uri)
}
