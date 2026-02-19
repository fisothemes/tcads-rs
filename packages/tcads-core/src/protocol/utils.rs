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
    if frame.header().command() != AmsCommand::AdsCommand {
        return Err(ProtocolError::UnexpectedAmsCommand {
            expected: AmsCommand::AdsCommand,
            got: frame.header().command(),
        });
    }

    let (ads_header, payload) = AdsHeader::parse_prefix(frame.payload()).map_err(AdsError::from)?;

    if ads_header.command_id() != expected_ads_cmd {
        return Err(ProtocolError::UnexpectedAdsCommand {
            expected: expected_ads_cmd,
            got: ads_header.command_id(),
        });
    }

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
