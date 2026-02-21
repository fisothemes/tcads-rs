use super::{ProtocolError, parse_ads_frame};
use crate::ads::{
    AdsCommand, AdsError, AdsHeader, AdsReturnCode, AdsState, DeviceState, StateFlag,
};
use crate::ams::{AmsAddr, AmsCommand};
use crate::io::AmsFrame;

/// A zero-copy view of an ADS Write Control Request (Command `0x0005`).
///
/// Borrows the additional data field directly from the [`AmsFrame`] that was parsed,
/// avoiding a copy. In the common case (PLC state changes) this data is empty, but
/// for custom ADS servers it may be non-trivial.
///
/// For cases where the request must be stored, sent across a channel, or used after
/// the frame is dropped, convert to [`AdsWriteControlRequestOwned`] via
/// [`into_owned`](Self::into_owned) or [`to_owned`](Self::to_owned).
///
/// # Usage
/// * **Client:** Sends this to control the state of the target device (e.g. starting/stopping a PLC).
/// * **Server:** Receives this, attempts the state transition, and responds with [`AdsWriteControlResponse`].
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWriteControl`](AdsCommand::AdsWriteControl) (`0x0005`)
/// * **ADS Payload Length:** 8 + n bytes (ADS State + Device State + Length + Data)
/// * **ADS Payload Layout:**
///   * **ADS State:** 2 bytes ([`AdsState`]) - The new target state (e.g., [`AdsState::Run`]).
///   * **Device State:** 2 bytes (u16) - The new device-specific state (usually 0).
///   * **Length:** 4 bytes (u32) - Length of the additional data.
///   * **Data:** n bytes - Additional data to transfer further information. Optional for current
///     ADS Devices (PLC, NC, ...).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlRequest<'a> {
    header: AdsHeader,
    ads_state: AdsState,
    device_state: DeviceState,
    data: &'a [u8],
}

impl<'a> AdsWriteControlRequest<'a> {
    /// The minimum size of the ADS payload (State + Device State + Length).
    pub const MIN_PAYLOAD_SIZE: usize = 8;

    /// Tries to parse a request from an AMS Frame.
    pub fn try_from_frame(frame: &'a AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }

    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the ADS state.
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the device state.
    pub fn device_state(&self) -> DeviceState {
        self.device_state
    }

    /// Returns the length of the additional data.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the additional data.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Converts this view into an owned [`AdsWriteControlRequestOwned`], copying the data.
    pub fn into_owned(self) -> AdsWriteControlRequestOwned {
        AdsWriteControlRequestOwned {
            header: self.header,
            ads_state: self.ads_state,
            device_state: self.device_state,
            data: self.data.to_vec(),
        }
    }

    /// Clones this view into an owned [`AdsWriteControlRequestOwned`], copying the data.
    pub fn to_owned(&self) -> AdsWriteControlRequestOwned {
        AdsWriteControlRequestOwned {
            header: self.header.clone(),
            ads_state: self.ads_state,
            device_state: self.device_state,
            data: self.data.to_vec(),
        }
    }

    /// Parses only the ADS payload portion.
    ///
    /// Returns the [ADS State](AdsState), [Device State](DeviceState), and additional data.
    pub fn parse_payload(payload: &[u8]) -> Result<(AdsState, DeviceState, &[u8]), ProtocolError> {
        if payload.len() < Self::MIN_PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::MIN_PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        let ads_state = AdsState::try_from(&payload[0..2]).map_err(AdsError::from)?;
        let device_state = DeviceState::from_le_bytes(payload[2..4].try_into().unwrap());
        let data_len = u32::from_le_bytes(payload[4..8].try_into().unwrap()) as usize;

        let data = if data_len > 0 {
            if payload.len() < Self::MIN_PAYLOAD_SIZE + data_len {
                return Err(AdsError::UnexpectedDataLength {
                    expected: Self::MIN_PAYLOAD_SIZE + data_len,
                    got: payload.len(),
                })?;
            }
            &payload[Self::MIN_PAYLOAD_SIZE..Self::MIN_PAYLOAD_SIZE + data_len]
        } else {
            &[]
        };

        Ok((ads_state, device_state, data))
    }
}

impl<'a> TryFrom<&'a AmsFrame> for AdsWriteControlRequest<'a> {
    type Error = ProtocolError;

    fn try_from(value: &'a AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsWriteControl, true)?;

        let (ads_state, device_state, data) = Self::parse_payload(data)?;

        Ok(Self {
            header,
            ads_state,
            device_state,
            data,
        })
    }
}

/// A fully owned ADS Write Control Request (Command `0x0005`).
///
/// Owns its data buffer, making it suitable for storage, sending across channels,
/// or constructing requests on a client to send to a device.
///
/// # Usage
/// * **Client:** Sends this to control the state of the target device (e.g. starting/stopping a PLC).
/// * **Server:** Receives this, attempts the state transition, and responds with [`AdsWriteControlResponse`].
///
/// Obtain one by:
/// * Calling [`AdsWriteControlRequestOwned::new`] to construct a request to send.
/// * Calling [`AdsWriteControlRequest::into_owned`] or [`AdsWriteControlRequest::to_owned`]
///   after parsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlRequestOwned {
    header: AdsHeader,
    ads_state: AdsState,
    device_state: DeviceState,
    data: Vec<u8>,
}

impl AdsWriteControlRequestOwned {
    /// The minimum size of the ADS payload (ADS State + Device State + Length).
    pub const MIN_PAYLOAD_SIZE: usize = AdsWriteControlRequest::MIN_PAYLOAD_SIZE;

    /// Creates a new Write Control Request without additional data.
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        ads_state: AdsState,
        device_state: DeviceState,
    ) -> Self {
        Self::with_data(
            target,
            source,
            invoke_id,
            ads_state,
            device_state,
            Vec::new(),
        )
    }

    /// Creates a new Write Control Request with additional data.
    ///
    /// Additional data to transfer further information. At this moment in time, it is not known
    /// what that data is 🙁.
    pub fn with_data(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: u32,
        ads_state: AdsState,
        device_state: DeviceState,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        let data = data.into();

        let header = AdsHeader::new(
            target,
            source,
            AdsCommand::AdsWriteControl,
            StateFlag::tcp_ads_request(),
            (Self::MIN_PAYLOAD_SIZE + data.len()) as u32,
            AdsReturnCode::Ok,
            invoke_id,
        );

        Self {
            header,
            ads_state,
            device_state,
            data,
        }
    }

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the ADS state.
    pub fn ads_state(&self) -> AdsState {
        self.ads_state
    }

    /// Returns the device state.
    pub fn device_state(&self) -> DeviceState {
        self.device_state
    }

    /// Returns the length of the additional data.
    pub fn length(&self) -> u32 {
        self.data.len() as u32
    }

    /// Returns the additional data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Borrows this request as a zero-copy [`AdsWriteControlRequest`].
    pub fn as_view(&self) -> AdsWriteControlRequest<'_> {
        AdsWriteControlRequest {
            header: self.header.clone(),
            ads_state: self.ads_state,
            device_state: self.device_state,
            data: &self.data,
        }
    }

    /// Consumes the request and converts it into an AMS Frame.
    pub fn into_frame(self) -> AmsFrame {
        AmsFrame::from(&self)
    }

    /// Serializes the request into an AMS Frame.
    pub fn to_frame(&self) -> AmsFrame {
        AmsFrame::from(self)
    }
}

impl From<&AdsWriteControlRequestOwned> for AmsFrame {
    fn from(value: &AdsWriteControlRequestOwned) -> Self {
        let mut payload = Vec::with_capacity(
            AdsHeader::LENGTH + AdsWriteControlRequestOwned::MIN_PAYLOAD_SIZE + value.data.len(),
        );

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.ads_state.to_bytes());
        payload.extend_from_slice(&value.device_state.to_le_bytes());
        payload.extend_from_slice(&(value.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&value.data);

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsWriteControlRequestOwned> for AmsFrame {
    fn from(value: AdsWriteControlRequestOwned) -> Self {
        AmsFrame::from(&value)
    }
}

impl<'a> From<AdsWriteControlRequest<'a>> for AdsWriteControlRequestOwned {
    fn from(value: AdsWriteControlRequest<'a>) -> Self {
        value.into_owned()
    }
}

impl<'a> From<&'a AdsWriteControlRequestOwned> for AdsWriteControlRequest<'a> {
    fn from(value: &'a AdsWriteControlRequestOwned) -> Self {
        value.as_view()
    }
}

/// Represents an ADS Write Control Response (Command `0x0005`).
///
/// This is the reply sent by the ADS device indicating the success or failure of the state
/// change operation.
///
/// # Usage
/// * **Server:** Sends this to acknowledge a state change request.
/// * **Client:** Receives this to confirm the operation was successful.
///
/// # Protocol Details
/// * **AMS Command:** [`AdsCommand`](AmsCommand::AdsCommand) (`0x0000`)
/// * **ADS Command:** [`AdsWriteControl`](AdsCommand::AdsWriteControl) (`0x0005`)
/// * **ADS Payload Length:** 4 bytes
/// * **ADS Payload Layout:**
///   * **Result Code:** 4 bytes ([`AdsReturnCode`])
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdsWriteControlResponse {
    header: AdsHeader,
    result: AdsReturnCode,
}

impl AdsWriteControlResponse {
    /// Size of the ADS Write Control Response body.
    pub const PAYLOAD_SIZE: usize = 4;

    /// Creates a new Write Control Response.
    pub fn new(target: AmsAddr, source: AmsAddr, invoke_id: u32, result: AdsReturnCode) -> Self {
        Self {
            header: AdsHeader::new(
                target,
                source,
                AdsCommand::AdsWriteControl,
                StateFlag::tcp_ads_response(),
                Self::PAYLOAD_SIZE as u32,
                result,
                invoke_id,
            ),
            result,
        }
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

    /// Returns the ADS header.
    pub fn header(&self) -> &AdsHeader {
        &self.header
    }

    /// Returns the [ADS Return Code](AdsReturnCode).
    pub fn result(&self) -> AdsReturnCode {
        self.result
    }

    /// Parses only the ADS payload portion.
    ///
    /// Returns the [ADS Return Code](AdsReturnCode).
    pub fn parse_payload(payload: &[u8]) -> Result<AdsReturnCode, ProtocolError> {
        if payload.len() != Self::PAYLOAD_SIZE {
            return Err(AdsError::UnexpectedDataLength {
                expected: Self::PAYLOAD_SIZE,
                got: payload.len(),
            })?;
        }

        Ok(AdsReturnCode::try_from_slice(payload).map_err(AdsError::from)?)
    }
}

impl From<&AdsWriteControlResponse> for AmsFrame {
    fn from(value: &AdsWriteControlResponse) -> Self {
        let mut payload =
            Vec::with_capacity(AdsHeader::LENGTH + AdsWriteControlResponse::PAYLOAD_SIZE);

        payload.extend_from_slice(&value.header.to_bytes());
        payload.extend_from_slice(&value.result.to_bytes());

        AmsFrame::new(AmsCommand::AdsCommand, payload)
    }
}

impl From<AdsWriteControlResponse> for AmsFrame {
    fn from(value: AdsWriteControlResponse) -> Self {
        AmsFrame::from(&value)
    }
}

impl TryFrom<&AmsFrame> for AdsWriteControlResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        let (header, data) = parse_ads_frame(value, AdsCommand::AdsWriteControl, false)?;

        Ok(Self {
            header,
            result: Self::parse_payload(data)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ams::AmsNetId;

    fn make_addrs() -> (AmsAddr, AmsAddr) {
        let target = AmsAddr::new(AmsNetId::new(192, 168, 0, 1, 1, 1), 851);
        let source = AmsAddr::new(AmsNetId::new(172, 16, 0, 1, 1, 1), 30000);
        (target, source)
    }

    #[test]
    fn test_request_no_data_roundtrip() {
        let (target, source) = make_addrs();

        let owned = AdsWriteControlRequestOwned::new(target, source, 1, AdsState::Run, 0);
        let frame = owned.to_frame();

        let view = AdsWriteControlRequest::try_from(&frame).expect("Should parse");

        assert_eq!(view.ads_state(), AdsState::Run);
        assert_eq!(view.device_state(), 0);
        assert!(view.data().is_empty());
        assert!(view.header().state_flags().is_request());
    }

    #[test]
    fn test_request_with_data_zero_copy() {
        let (target, source) = make_addrs();
        let extra = vec![0x01, 0x02, 0x03, 0x04];

        let owned = AdsWriteControlRequestOwned::with_data(
            target,
            source,
            7,
            AdsState::Stop,
            0,
            extra.clone(),
        );
        let frame = owned.to_frame();

        let view = AdsWriteControlRequest::try_from(&frame).expect("Should parse");

        assert_eq!(view.ads_state(), AdsState::Stop);
        assert_eq!(view.data(), extra.as_slice());

        // Verify zero-copy: data points into the frame payload
        // Offset: AdsHeader (32) + AdsState (2) + DeviceState (2) + Length (4) = 40
        let frame_payload_ptr = frame.payload().as_ptr();
        let view_data_ptr = view.data().as_ptr();
        assert_eq!(view_data_ptr, unsafe { frame_payload_ptr.add(40) });
    }

    #[test]
    fn test_view_into_owned() {
        let (target, source) = make_addrs();
        let extra = vec![0xAA, 0xBB];

        let owned = AdsWriteControlRequestOwned::with_data(
            target,
            source,
            1,
            AdsState::Config,
            0,
            extra.clone(),
        );
        let frame = owned.to_frame();

        let view = AdsWriteControlRequest::try_from(&frame).expect("Should parse");
        let converted = view.into_owned();

        assert_eq!(converted.ads_state(), AdsState::Config);
        assert_eq!(converted.data(), extra.as_slice());
    }

    #[test]
    fn test_owned_as_view() {
        let (target, source) = make_addrs();

        let owned = AdsWriteControlRequestOwned::new(target, source, 1, AdsState::Run, 0);
        let view = owned.as_view();

        assert_eq!(view.ads_state(), AdsState::Run);
        assert!(view.data().is_empty());
    }

    #[test]
    fn test_owned_roundtrip_via_frame() {
        let (target, source) = make_addrs();
        let extra = vec![0x01, 0x02];

        let original = AdsWriteControlRequestOwned::with_data(
            target,
            source,
            42,
            AdsState::Stop,
            0,
            extra.clone(),
        );
        let frame = original.to_frame();

        let view = AdsWriteControlRequest::try_from(&frame).expect("Should parse");
        let roundtripped = view.into_owned();

        assert_eq!(roundtripped.ads_state(), original.ads_state());
        assert_eq!(roundtripped.device_state(), original.device_state());
        assert_eq!(roundtripped.data(), original.data());
        assert_eq!(
            roundtripped.header().invoke_id(),
            original.header().invoke_id()
        );
    }

    #[test]
    fn test_response_roundtrip() {
        let (target, source) = make_addrs();

        let response = AdsWriteControlResponse::new(target, source, 1, AdsReturnCode::Ok);
        let frame = response.to_frame();

        let decoded = AdsWriteControlResponse::try_from(&frame).expect("Should parse");

        assert_eq!(decoded.result(), AdsReturnCode::Ok);
        assert!(decoded.header().state_flags().is_response());
    }

    #[test]
    fn test_response_uses_parse_ads_frame() {
        // Feeding a request frame to the response parser should fail
        let (target, source) = make_addrs();
        let req = AdsWriteControlRequestOwned::new(target, source, 1, AdsState::Run, 0);
        let frame = req.to_frame();

        let err = AdsWriteControlResponse::try_from(&frame).unwrap_err();
        assert!(matches!(err, ProtocolError::Ads(_)));
    }
}
