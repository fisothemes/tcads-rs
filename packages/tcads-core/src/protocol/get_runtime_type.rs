use super::{ProtocolError, validate_ams_command};
use crate::AmsFrame;
use crate::ams::{AmsCommand, AmsError, RuntimeType};

/// Represents an AMS Get Runtime Type Request (Command `0x1003`).
///
/// This command is sent to the local AMS Router to determine whether the
/// TwinCAT runtime is a real-time or user-mode system.
///
/// # Usage
/// * **Client:** Sends this to identify the type of TwinCAT runtime on the target machine.
/// * **Server/Router:** Receives this and responds with its configured runtime type.
///
/// # Protocol Details
/// * **Command ID:** `0x1003`
/// * **Payload Length:** 0 bytes (empty body)
///
/// # Warning
/// This command is only supported by a local AMS router. The remote router does not support
/// this command and will close the TCP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GetRuntimeTypeRequest;

impl GetRuntimeTypeRequest {
    /// Creates a frame for this request.
    pub fn into_frame() -> AmsFrame {
        AmsFrame::empty(AmsCommand::GetRuntimeType)
    }
}

impl From<GetRuntimeTypeRequest> for AmsFrame {
    fn from(_: GetRuntimeTypeRequest) -> Self {
        AmsFrame::empty(AmsCommand::GetRuntimeType)
    }
}

impl TryFrom<&AmsFrame> for GetRuntimeTypeRequest {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        validate_ams_command(value, AmsCommand::GetRuntimeType)?;

        if value.header().length() != 0 {
            return Err(ProtocolError::UnexpectedLength {
                expected: 0,
                got: value.header().length() as usize,
            });
        }

        if !value.payload().is_empty() {
            return Err(ProtocolError::UnexpectedLength {
                expected: 0,
                got: value.payload().len(),
            });
        }

        Ok(Self)
    }
}

/// Represents an AMS Get Runtime Type Response (Command `0x1003`).
///
/// This is the reply sent by the local AMS Router containing its runtime type.
///
/// # Usage
/// * **Client:** Receives this to determine the runtime type of the TwinCAT system.
/// * **Server/Router:** Sends this in response to a [`GetRuntimeTypeRequest`].
///
/// # Protocol Details
/// * **Command ID:** `0x1003`
/// * **Payload Length:** 4 bytes
/// * **Payload:** 32-bit little-endian integer representing the [`RuntimeType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GetRuntimeTypeResponse {
    runtime_type: RuntimeType,
}

impl GetRuntimeTypeResponse {
    /// Creates a new response.
    pub fn new(runtime_type: RuntimeType) -> Self {
        Self { runtime_type }
    }

    /// Returns the [`RuntimeType`] of the TwinCAT system.
    pub fn runtime_type(&self) -> RuntimeType {
        self.runtime_type
    }

    /// Consumes the response and converts it into a raw [`AmsFrame`].
    pub fn into_frame(self) -> AmsFrame {
        self.into()
    }

    /// Creates a raw [`AmsFrame`] from the response.
    pub fn to_frame(&self) -> AmsFrame {
        self.into()
    }

    /// Attempts to parse a [`GetRuntimeTypeResponse`] from an [`AmsFrame`].
    pub fn try_from_frame(frame: &AmsFrame) -> Result<Self, ProtocolError> {
        Self::try_from(frame)
    }
}

impl From<GetRuntimeTypeResponse> for AmsFrame {
    fn from(value: GetRuntimeTypeResponse) -> Self {
        let runtime: u32 = value.runtime_type.into();
        AmsFrame::new(AmsCommand::GetRuntimeType, runtime.to_le_bytes())
    }
}

impl From<&GetRuntimeTypeResponse> for AmsFrame {
    fn from(value: &GetRuntimeTypeResponse) -> Self {
        (*value).into()
    }
}

impl TryFrom<&AmsFrame> for GetRuntimeTypeResponse {
    type Error = ProtocolError;

    fn try_from(value: &AmsFrame) -> Result<Self, Self::Error> {
        validate_ams_command(value, AmsCommand::GetRuntimeType)?;

        if value.header().length() as usize != RuntimeType::LENGTH {
            return Err(ProtocolError::UnexpectedLength {
                expected: RuntimeType::LENGTH,
                got: value.payload().len(),
            });
        }

        let runtime_type = RuntimeType::try_from_slice(value.payload()).map_err(AmsError::from)?;

        Ok(Self { runtime_type })
    }
}

impl TryFrom<AmsFrame> for GetRuntimeTypeResponse {
    type Error = ProtocolError;

    fn try_from(value: AmsFrame) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_produces_empty_payload() {
        let frame = GetRuntimeTypeRequest::into_frame();
        assert_eq!(frame.header().command(), AmsCommand::GetRuntimeType);
        assert_eq!(frame.header().length(), 0);
        assert!(frame.payload().is_empty());
    }

    #[test]
    fn response_roundtrip_rt() {
        let resp = GetRuntimeTypeResponse::new(RuntimeType::Rt);
        let frame = resp.to_frame();
        let decoded = GetRuntimeTypeResponse::try_from(&frame).unwrap();
        assert_eq!(decoded.runtime_type(), RuntimeType::Rt);
        assert_eq!(frame.payload(), &[0, 0, 0, 0]);
    }

    #[test]
    fn response_roundtrip_um() {
        let resp = GetRuntimeTypeResponse::new(RuntimeType::Um);
        let frame = resp.to_frame();
        let decoded = GetRuntimeTypeResponse::try_from(&frame).unwrap();
        assert_eq!(decoded.runtime_type(), RuntimeType::Um);
        assert_eq!(frame.payload(), &[1, 0, 0, 0]);
    }

    #[test]
    fn response_roundtrip_unknown() {
        let resp = GetRuntimeTypeResponse::new(RuntimeType::Unknown(42));
        let frame = resp.to_frame();
        let decoded = GetRuntimeTypeResponse::try_from(&frame).unwrap();
        assert_eq!(decoded.runtime_type(), RuntimeType::Unknown(42));
    }

    #[test]
    fn wrong_command_rejected() {
        let frame = AmsFrame::new(AmsCommand::GetLocalNetId, [0u8; 4]);
        let err = GetRuntimeTypeResponse::try_from(&frame).unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedAmsCommand {
                expected: AmsCommand::GetRuntimeType,
                ..
            }
        ));
    }

    #[test]
    fn wrong_length_rejected() {
        let frame = AmsFrame::new(AmsCommand::GetRuntimeType, [0u8; 2]);
        let err = GetRuntimeTypeResponse::try_from(&frame).unwrap_err();
        println!("{:?}", err);
        assert!(matches!(
            err,
            ProtocolError::UnexpectedLength {
                expected: RuntimeType::LENGTH,
                got: 2
            }
        ));
    }
}
