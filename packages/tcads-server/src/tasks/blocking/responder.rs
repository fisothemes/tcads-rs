use std::sync::mpsc::Sender;
use tcads_core::ads::StateFlag;
use tcads_core::protocol::AdsReadDeviceInfoResponse;
use tcads_core::{
    AdsCommand, AdsDeviceVersion, AdsHeader, AdsReturnCode, AmsAddr, AmsCommand, AmsFrame, InvokeId,
};

pub struct AdsReadDeviceInfoResponder {
    target: AmsAddr,
    source: AmsAddr,
    invoke_id: InvokeId,
    write_tx: Sender<AmsFrame>,
    answered: bool,
}

impl AdsReadDeviceInfoResponder {
    pub fn new(
        target: AmsAddr,
        source: AmsAddr,
        invoke_id: InvokeId,
        write_tx: Sender<AmsFrame>,
    ) -> Self {
        Self {
            target,
            source,
            invoke_id,
            write_tx,
            answered: false,
        }
    }

    pub fn target(&self) -> AmsAddr {
        self.target
    }

    pub fn source(&self) -> AmsAddr {
        self.source
    }
    pub fn invoke_id(&self) -> InvokeId {
        self.invoke_id
    }

    pub fn ok(
        mut self,
        version: impl Into<AdsDeviceVersion>,
        name: impl AsRef<str>,
    ) -> crate::Result<()> {
        let frame = AdsReadDeviceInfoResponse::try_new(
            self.target,
            self.source,
            self.invoke_id,
            AdsReturnCode::Ok,
            version.into(),
            name,
        )?
        .to_frame();

        self.answered = true;
        self.write_tx.send(frame)?;

        Ok(())
    }

    pub fn err(mut self, code: AdsReturnCode) -> crate::Result<()> {
        let code = if code == AdsReturnCode::Ok {
            AdsReturnCode::AdsErrDeviceError
        } else {
            code
        };

        let frame = self.header_only_resp_frame(code)?;
        self.answered = true;
        self.write_tx.send(frame)?;

        Ok(())
    }

    fn header_only_resp_frame(&self, code: AdsReturnCode) -> crate::Result<AmsFrame> {
        let mut bytes = AdsHeader::new(
            self.target,
            self.source,
            AdsCommand::AdsReadDeviceInfo,
            StateFlag::RESPONSE,
            AdsReturnCode::LENGTH as u32,
            code,
            self.invoke_id,
        )
        .to_bytes()
        .to_vec();

        bytes.extend_from_slice(&code.to_bytes());

        Ok(AmsFrame::new(AmsCommand::AdsCommand, bytes))
    }
}

impl From<(AdsHeader, Sender<AmsFrame>)> for AdsReadDeviceInfoResponder {
    fn from((header, write_tx): (AdsHeader, Sender<AmsFrame>)) -> Self {
        Self::new(
            *header.target(),
            *header.source(),
            header.invoke_id(),
            write_tx,
        )
    }
}

impl Drop for AdsReadDeviceInfoResponder {
    fn drop(&mut self) {
        if !self.answered {
            if let Ok(frame) = self.header_only_resp_frame(AdsReturnCode::AdsErrDeviceAborted) {
                let _ = self.write_tx.send(frame);
            }
        }
    }
}
