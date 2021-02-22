pub struct MockTime(DateTime<Utc>);
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "Timestamp", into = "Timestamp")]

impl Protobuf<Timestamp> for MockTime {}

impl TryFrom<Timestamp> for MockTime {
    type Error = Infallible;

    fn try_from(value: Timestamp) -> Result<Self, Self::Error> {
        // prost_types::Timestamp has a SystemTime converter but
        // tendermint_proto::Timestamp can be JSON-encoded
        let prost_value = prost_types::Timestamp {
            seconds: value.seconds,
            nanos: value.nanos,
        };

        Ok(SystemTime::from(prost_value).into())
    }
}

impl From<Time> for Timestamp {
    fn from(value: MockTime) -> Self {
        // prost_types::Timestamp has a SystemTime converter but
        // tendermint_proto::Timestamp can be JSON-encoded
        let prost_value = prost_types::Timestamp::from(value.to_system_time().unwrap());
        Timestamp {
            seconds: prost_value.seconds,
            nanos: prost_value.nanos,
        }
    }
}

impl MockTime {
    /// Get [`Time`] value representing the current wall clock time
    pub fn now() -> Self {
        MockTime(Utc::now())
    }

    /// Get the [`UNIX_EPOCH`] time ("1970-01-01 00:00:00 UTC") as a [`Time`]
    pub fn unix_epoch() -> Self {
        UNIX_EPOCH.into()
    }

    /// Calculate the amount of time which has passed since another [`Time`]
    /// as a [`std::time::Duration`]
    pub fn duration_since(&self, other: MockTime) -> Result<Duration, Error> {
        self.0
            .signed_duration_since(other.0)
            .to_std()
            .map_err(|_| Kind::OutOfRange.into())
    }

    /// Parse [`Time`] from an RFC 3339 date
    pub fn parse_from_rfc3339(s: &str) -> Result<MockTime, Error> {
        Ok(MockTime(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc)))
    }

    /// Return an RFC 3339 and ISO 8601 date and time string with 6 subseconds digits and Z.
    pub fn to_rfc3339(&self) -> String {
        timestamp::to_rfc3339_nanos(&self.0)
    }

    /// Convert [`Time`] to [`SystemTime`]
    pub fn to_system_time(&self) -> Result<SystemTime, Error> {
        let duration_since_epoch = self.duration_since(Self::unix_epoch())?;
        Ok(UNIX_EPOCH + duration_since_epoch)
    }
}

impl fmt::Display for MockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_rfc3339())
    }
}

impl FromStr for MockTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MockTime::parse_from_rfc3339(s)
    }
}

impl From<DateTime<Utc>> for Time {
    fn from(t: DateTime<Utc>) -> Time {
        MockTime(t)
    }
}

impl From<MockTime> for DateTime<Utc> {
    fn from(t: MockTime) -> DateTime<Utc> {
        t.0
    }
}

impl From<SystemTime> for MockTime {
    fn from(t: SystemTime) -> MockTime {
        MockTime(t.into())
    }
}

impl From<MockTime> for SystemTime {
    fn from(t: MockTime) -> SystemTime {
        t.to_system_time().unwrap()
    }
}

impl Add<Duration> for MockTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        let st: SystemTime = self.into();
        (st + rhs).into()
    }
}

impl Sub<Duration> for MockTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        let st: SystemTime = self.into();
        (st - rhs).into()
    }
}

/// Parse [`Time`] from a type
pub trait ParseTimestamp {
    /// Parse [`Time`], or return an [`Error`] if parsing failed
    fn parse_timestamp(&self) -> Result<MockTime, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        const DATES: &[&str] = &[
            "2020-09-14T16:33:54.21191421Z",
            "2020-09-14T16:33:00Z",
            "2020-09-14T16:33:00.1Z",
            "2020-09-14T16:33:00.211914212Z",
            "1970-01-01T00:00:00Z",
            "2021-01-07T20:25:56.0455760Z",
            "2021-01-07T20:25:57.039219Z",
            "2021-01-07T20:25:58.03562100Z",
            "2021-01-07T20:25:59.000955200Z",
            "2021-01-07T20:26:04.0121030Z",
            "2021-01-07T20:26:05.005096Z",
            "2021-01-07T20:26:09.08488400Z",
            "2021-01-07T20:26:11.0875340Z",
            "2021-01-07T20:26:12.078268Z",
            "2021-01-07T20:26:13.08074100Z",
            "2021-01-07T20:26:15.079663000Z",
        ];

        for input in DATES {
            let initial_time: MockTime = input.parse().unwrap();
            let encoded_time = serde_json::to_value(&initial_time).unwrap();
            let decoded_time = serde_json::from_value(encoded_time.clone()).unwrap();

            assert_eq!(initial_time, decoded_time);
        }
    }
}