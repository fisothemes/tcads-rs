use super::ProtocolError;
use crate::ads::{AdsCommand, AdsError, AdsHeader, StateFlag, StateFlagError};
use crate::ams::AmsCommand;
use crate::io::AmsFrame;

/// Parses an AMS Frame and checks if it's an ADS frame.
pub fn parse_ads_frame(
    frame: &AmsFrame,
    expected_ads_cmd: AdsCommand,
    is_request: bool,
) -> Result<(AdsHeader, &[u8]), ProtocolError> {
    validate_ams_command(frame, AmsCommand::AdsCommand)?;

    let (ads_header, payload) = AdsHeader::parse_prefix(frame.payload()).map_err(AdsError::from)?;

    validate_ads_command(&ads_header, expected_ads_cmd)?;

    let flags = ads_header.state_flags();

    if is_request && !flags.is_request() {
        return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
            expected: vec![StateFlag::tcp_ads_request(), StateFlag::udp_ads_request()],
            got: flags,
        })
        .into());
    }
    if !is_request && !flags.is_response() {
        return Err(AdsError::from(StateFlagError::UnexpectedStateFlag {
            expected: vec![StateFlag::tcp_ads_response(), StateFlag::udp_ads_response()],
            got: flags,
        })
        .into());
    }

    Ok((ads_header, payload))
}

/// Validates the AMS frame's command.
pub fn validate_ams_command(frame: &AmsFrame, expected: AmsCommand) -> Result<(), ProtocolError> {
    if frame.header().command() != expected {
        return Err(ProtocolError::UnexpectedAmsCommand {
            expected,
            got: frame.header().command(),
        });
    }
    Ok(())
}

/// Validates the ADS frame's command.
pub fn validate_ads_command(header: &AdsHeader, expected: AdsCommand) -> Result<(), ProtocolError> {
    if header.command_id() != expected {
        return Err(ProtocolError::UnexpectedAdsCommand {
            expected,
            got: header.command_id(),
        });
    }
    Ok(())
}
