use super::ProtocolError;
use crate::ads::{
    AdsCommand, AdsDeviceVersion, AdsError, AdsHeader, AdsReturnCode, AdsString, StateFlag,
    StateFlagError,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;
use std::borrow::Cow;

/// Represents an ADS Read Device Info Request (Command `0x0001`).
///
/// This command is sent to an ADS device to query its name and version information.
///
/// # Usage
/// * **Client:** Sends this to identify the target device (e.g. "Plc30 App").
/// * **Server:** Receives this and responds with [`AdsReadDeviceInfoResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadDeviceInfo`](AdsCommand::AdsReadDeviceInfo) (`0x0001`)
/// * **ADS Payload Length:** 0 bytes (Body is empty)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadDeviceInfoRequest {
    header: AdsHeader,
}

impl AdsReadDeviceInfoRequest {
    /// Creates a new Read Device Info Request over TCP.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadDeviceInfo,
            StateFlag::tcp_ads_request(),
            0,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header }
    }

    /// Creates a new Read Device Info Request over UDP.
    pub fn new_udp(target: AmsAddr, source: AmsAddr, invoke_id: u32) -> Self {
        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadDeviceInfo,
            StateFlag::udp_ads_request(),
            0,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self { header }
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Tries to parse a request from an AMS Frame.
    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Consumes the request and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(self)
    }

    /// Serializes the request into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }
}

impl From<&AdsReadDeviceInfoRequest> for AmsFrame {
    fn from(request: &AdsReadDeviceInfoRequest) -> Self {
        AmsFrame::new(AmsCommand::AdsCommand, request.header.to_bytes())
    }
}

impl From<AdsReadDeviceInfoRequest> for AmsFrame {
    fn from(request: AdsReadDeviceInfoRequest) -> Self {
        AmsFrame::from(&request)
    }
}

impl TryFrom<&AmsFrame> for AdsReadDeviceInfoRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::AdsCommand {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::AdsCommand,
                got: header.command(),
            });
        }

        let (header, data) = AdsHeader::parse_prefix(value.payload()).map_err(AdsError::from)?;

        if header.command_id() != AdsCommand::AdsReadDeviceInfo {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsReadDeviceInfo,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_request() {
            return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_request(), StateFlag::udp_ads_request()],
                got: header.state_flags(),
            }))?;
        }

        if !data.is_empty() {
            return Err(AdsError::UnexpectedDataLength {
                expected: 0,
                got: data.len(),
            })?;
        }

        Ok(Self { header })
    }
}

/// Represents an ADS Read Device Info Response (Command `0x0001`).
///
/// This is the reply containing the name and version of the ADS device.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsReadDeviceInfo`](AdsCommand::AdsReadDeviceInfo) (`0x0001`)
/// * **ADS Payload Length:** 24 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
///   * **Version:** 4 bytes ([`AdsDeviceVersion`])
///   * **Device Name:** 16 bytes (Fixed-length string, null-terminated)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsReadDeviceInfoResponse {
    header: AdsHeader,
    result: AdsReturnCode,
    version: AdsDeviceVersion,
    device_name: AdsString<16>,
}

impl AdsReadDeviceInfoResponse {
    /// Size of the ADS Read Device Info Response body.
    pub const PAYLOAD_SIZE: usize = 24;

    /// Creates a new Read Device Info Response over TCP.
    ///
    /// # Note
    ///
    /// The device name is limited to 16 bytes and must use character valid in
    /// the Windows-1252 (CP1252) encoding.
    pub fn try_new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        result: AdsReturnCode,
        version: AdsDeviceVersion,
        device_name: impl AsRef<str>,
    ) -> Result<Self, ProtocolError> {
        let name = AdsString::try_from(device_name.as_ref()).map_err(AdsError::from)?;

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsReadDeviceInfo,
            StateFlag::tcp_ads_response(),
            Self::PAYLOAD_SIZE as u32,
            result,
            invoke_id,
        );

        Ok(Self {
            header,
            result,
            version,
            device_name: name,
        })
    }

    /// Tries to parse a response from an AMS Frame.
    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    /// Consumes the response and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the response into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }

    /// Returns the ADS return code.
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Returns the ADS device version.
    pub fn version(&self) -> AdsDeviceVersion {
        self.version
    }

    /// Returns the ADS device name.
    pub fn device_name(&self) -> Cow<'_, str> {
        self.device_name.as_str()
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Parses the response payload into a tuple of return code, version, and device name.
    pub fn parse_payload(
        payload: &[u8],
    ) -> Result<(AdsReturnCode, AdsDeviceVersion, AdsString<16>), AdsError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let result = AdsReturnCode::try_from_slice(&payload[0..4]).map_err(AdsError::from)?;
        let version = AdsDeviceVersion::try_from_slice(&payload[4..8]).map_err(AdsError::from)?;
        let raw_name: [u8; 16] = payload[8..24].try_into().unwrap();
        let device_name = AdsString::from(raw_name);

        Ok((result, version, device_name))
    }
}

impl From<&AdsReadDeviceInfoResponse> for AmsFrame {
    fn from(value: &AdsReadDeviceInfoResponse) -> Self {
        let mut payload =
            Vec::with_capacity(AdsHeader::LENGTH + AdsReadDeviceInfoResponse::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());
        payload.extend_from_slice(&value.version.to_bytes());
        payload.extend_from_slice(value.device_name.as_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsReadDeviceInfoResponse> for AmsFrame {
    fn from(value: AdsReadDeviceInfoResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsReadDeviceInfoResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let header = value.header();

        if header.command() != AmsCommand::AdsCommand {
            return Err(ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::AdsCommand,
                got: header.command(),
            });
        };

        let (header, data) = AdsHeader::parse_prefix(value.payload()).map_err(AdsError::from)?;

        if header.command_id() != AdsCommand::AdsReadDeviceInfo {
            return Err(ProtocolError::UnexpectedAdsCommand {
                expected: AdsCommand::AdsReadDeviceInfo,
                got: header.command_id(),
            });
        }

        if !header.state_flags().is_response() {
            return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
                expected: vec![StateFlag::tcp_ads_response(), StateFlag::udp_ads_response()],
                got: header.state_flags(),
            }))?;
        }

        let (result, version, device_name) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            result,
            version,
            device_name,
        })
    }
}

impl TryFrom<AmsFrame> for AdsReadDeviceInfoResponse {
    type Error = ProtocolError;
    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsNetId;

    #[test]
    fn test_device_info_response_roundtrip() {
        let target = AmsAddr::new(AmsNetId::new(1, 2, 3, 4, 5, 6), 851);
        let source = AmsAddr::new(AmsNetId::new(6, 5, 4, 3, 2, 1), 1000);
        let invoke_id = 100;
        let version = AdsDeviceVersion::new(3, 1, 4024);

        let response = AdsReadDeviceInfoResponse::try_new(
            target,
            source,
            invoke_id,
            AdsReturnCode::Ok,
            version,
            "TC3 PLC",
        )
        .expect("Failed to create response");

        let frame = response.to_frame();
        let decoded = AdsReadDeviceInfoResponse::try_from(&frame).expect("Deserialization failed");

        assert_eq!(decoded.version().major(), 3);
        assert_eq!(decoded.version().build(), 4024);
        assert_eq!(decoded.device_name(), "TC3 PLC");
    }

    #[test]
    fn test_device_name_too_long() {
        let target = AmsAddr::default();
        let source = AmsAddr::default();
        let version = AdsDeviceVersion::default();

        let err = AdsReadDeviceInfoResponse::try_new(
            target,
            source,
            1,
            AdsReturnCode::Ok,
            version,
            "1234567890123456",
        );

        assert!(err.is_err());
    }
}
