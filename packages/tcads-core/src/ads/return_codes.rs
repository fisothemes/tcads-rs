use super::error::AdsReturnCodeError;

/// ADS Return Codes representing the result of an ADS operation.
///
/// See [Beckhoff ADS Specification (TE1000)](https://infosys.beckhoff.com/content/1033/tc3_ads_intro/374277003.html?id=4954945278371876402)
/// for reference
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdsReturnCode {
    #[error("No Error (0x00)")]
    Ok,

    // --- Global Error Codes (0x00 .. 0x1E) ---
    #[error("Internal error (0x1)")]
    ErrInternal,
    #[error("No real time (0x2)")]
    ErrNoRtime,
    #[error("Allocation locked – memory error (0x3)")]
    ErrAllocLockedMem,
    #[error(
        "Mailbox full – the ADS message could not be sent. \
        Reducing the number of ADS messages per cycle will help (0x4)"
    )]
    ErrInsertMailbox,
    #[error("Wrong HMSG (0x5)")]
    ErrWrongReceiveHMsg,
    #[error(
        "Target port not found – ADS server is not started, not reachable or not installed (0x6)"
    )]
    ErrTargetPortNotFound,
    #[error("Target computer not found – AMS route was not found (0x7)")]
    ErrTargetMachineNotFound,
    #[error("Unknown command ID (0x8)")]
    ErrUnknownCmdId,
    #[error("Invalid task ID (0x9)")]
    ErrBadTaskId,
    #[error("No IO (0x0A)")]
    ErrNoIo,
    #[error("Unknown AMS command (0xB)")]
    ErrUnknownAmsCmd,
    #[error("Win32 error (0xC)")]
    ErrWin32Error,
    #[error("Port not connected (0xD)")]
    ErrPortNotConnected,
    #[error("Invalid AMS length (0xE)")]
    ErrInvalidAmsLength,
    #[error("Invalid AMS Net ID (0xF)")]
    ErrInvalidAmsNetId,
    #[error("Installation level is too low –TwinCAT 2 license error (0x10)")]
    ErrLowInstLevel,
    #[error("No debugging available (0x11)")]
    ErrNoDebug,
    #[error("Port disabled – TwinCAT system service not started (0x12)")]
    ErrPortDisabled,
    #[error("Port already connected (0x13)")]
    ErrPortAlreadyConnected,
    #[error("AMS Sync Win32 error (0x14)")]
    ErrAmsSyncW32Error,
    #[error("AMS Sync Timeout (0x15)")]
    ErrAmsSyncTimeout,
    #[error("AMS Sync error (0x16)")]
    ErrAmsSyncError,
    #[error("No index map for AMS Sync available (0x17)")]
    ErrAmsSyncNoIndexInMap,
    #[error("Invalid AMS port (0x18)")]
    ErrInvalidAmsPort,
    #[error("No memory (0x19)")]
    ErrNoMemory,
    #[error("TCP send error (0x1A)")]
    ErrTcpSend,
    #[error("Host unreachable (0x1B)")]
    ErrHostUnreachable,
    #[error("Invalid AMS fragment (0x1C)")]
    ErrInvalidAmsFragment,
    #[error("TLS send error – secure ADS connection failed (0x1D)")]
    ErrTlsSend,
    #[error("Access denied – secure ADS access denied (0x1E)")]
    ErrAccessDenied,

    // --- Router Error Codes (0x500 .. 0x50D) ---
    #[error("Router: Locked memory cannot be allocated (0x500)")]
    RouterErrNoLockedMemory,
    #[error("Router: The router memory size could not be changed (0x501)")]
    RouterErrResizeMemory,
    #[error("Router: The mailbox has reached the maximum number of possible messages (0x502)")]
    RouterErrMailboxFull,
    #[error(
        "Router: The Debug mailbox has reached the maximum number of possible messages (0x503)"
    )]
    RouterErrDebugBoxFull,
    #[error("Router: The port type is unknown (0x504)")]
    RouterErrUnknownPortType,
    #[error("Router: The router is not initialized (0x505)")]
    RouterErrNotInitialized,
    #[error("Router: The port number is already assigned (0x506)")]
    RouterErrPortAlreadyInUse,
    #[error("Router: The port is not registered (0x507)")]
    RouterErrNotRegistered,
    #[error("Router: The maximum number of ports has been reached (0x508)")]
    RouterErrNoMoreQueues,
    #[error("Router: The port is invalid (0x509)")]
    RouterErrInvalidPort,
    #[error("Router: The router is not active (0x50A)")]
    RouterErrNotActivated,
    #[error("Router: The mailbox has reached the maximum number for fragmented messages (0x50B)")]
    RouterErrFragmentBoxFull,
    #[error("Router: A fragment timeout has occurred (0x50C)")]
    RouterErrFragmentTimeout,
    #[error("Router: The port is removed (0x50D)")]
    RouterErrToBeRemoved,

    // --- General ADS Error Codes (0x700 .. 0x756) ---
    #[error("ADS: General device error (0x700)")]
    AdsErrDeviceError,
    #[error("ADS: Service is not supported by the server (0x701)")]
    AdsErrDeviceSrvNotSupp,
    #[error("ADS: Invalid index group (0x702)")]
    AdsErrDeviceInvalidGrp,
    #[error("ADS: Invalid index offset (0x703)")]
    AdsErrDeviceInvalidOffset,
    #[error(
        "ADS: Reading or writing not permitted. \
        Several causes are possible. \
        For example, an incorrect password was entered when creating routes (0x704)"
    )]
    AdsErrDeviceInvalidAccess,
    #[error("ADS: Parameter size not correct (0x705)")]
    AdsErrDeviceInvalidSize,
    #[error("ADS: Invalid data values (0x706)")]
    AdsErrDeviceInvalidData,
    #[error("ADS: Device is not ready to operate (0x707)")]
    AdsErrDeviceNotReady,
    #[error("ADS: Device is busy (0x708)")]
    AdsErrDeviceBusy,
    #[error(
        "ADS: Invalid operating system context. \
        This can result from use of ADS blocks in different tasks. \
        It may be possible to resolve this through multitasking synchronization in the PLC (0x709)"
    )]
    AdsErrDeviceInvalidContext,
    #[error("ADS: Insufficient memory (0x70A)")]
    AdsErrDeviceNoMemory,
    #[error("ADS: Invalid parameter values (0x70B)")]
    AdsErrDeviceInvalidParm,
    #[error("ADS: Not found (files, ...) (0x70C)")]
    AdsErrDeviceNotFound,
    #[error("ADS: Syntax error in file or command (0x70D)")]
    AdsErrDeviceSyntax,
    #[error("ADS: Objects do not match (0x70E)")]
    AdsErrDeviceIncompatible,
    #[error("ADS: Object already exists (0x70F)")]
    AdsErrDeviceExists,
    #[error("ADS: Symbol not found (0x710)")]
    AdsErrDeviceSymbolNotFound,
    #[error(
        "ADS: Invalid symbol version. This can occur due to an online change. \
        Create a new handle (0x711)"
    )]
    AdsErrDeviceSymbolVersionInvalid,
    #[error("ADS: Device (server) is in invalid state (0x712)")]
    AdsErrDeviceInvalidState,
    #[error("ADS: AdsTransMode not supported (0x713)")]
    AdsErrDeviceTransModeNotSupp,
    #[error("ADS: Notification handle is invalid (0x714)")]
    AdsErrDeviceNotifyHndInvalid,
    #[error("ADS: Notification client not registered (0x715)")]
    AdsErrDeviceClientUnknown,
    #[error("ADS: No further handle available (0x716)")]
    AdsErrDeviceNoMoreHdls,
    #[error("ADS: Notification size too large (0x717)")]
    AdsErrDeviceInvalidWatchSize,
    #[error("ADS: Device not initialized (0x718)")]
    AdsErrDeviceNotInit,
    #[error("ADS: Device has a timeout (0x719)")]
    AdsErrDeviceTimeout,
    #[error("ADS: Interface query failed (0x71A)")]
    AdsErrDeviceNoInterface,
    #[error("ADS: Wrong interface requested (0x71B)")]
    AdsErrDeviceInvalidInterface,
    #[error("ADS: Class ID is invalid (0x71C)")]
    AdsErrDeviceInvalidClsId,
    #[error("ADS: IObject ID is invalid (0x71D)")]
    AdsErrDeviceInvalidObjId,
    #[error("ADS: Request pending (0x71E)")]
    AdsErrDevicePending,
    #[error("ADS: Request is aborted (0x71F)")]
    AdsErrDeviceAborted,
    #[error("ADS: Signal warning (0x720)")]
    AdsErrDeviceWarning,
    #[error("ADS: Invalid array index (0x721)")]
    AdsErrDeviceInvalidArrayIdx,
    #[error("ADS: Symbol not active (0x722)")]
    AdsErrDeviceSymbolNotActive,
    #[error(
        "ADS: Access denied.\n\
        Several causes are possible. \
        For example, a unidirectional ADS route is used in the opposite direction. (0x723)"
    )]
    AdsErrDeviceAccessDenied,
    #[error("ADS: Missing license (0x724)")]
    AdsErrDeviceLicenseNotFound,
    #[error("ADS: License expired (0x725)")]
    AdsErrDeviceLicenseExpired,
    #[error("ADS: License exceeded (0x726)")]
    AdsErrDeviceLicenseExceeded,
    #[error("ADS: Invalid license (0x727)")]
    AdsErrDeviceLicenseInvalid,
    #[error("ADS: License problem: System ID is invalid (0x728)")]
    AdsErrDeviceLicenseSystemId,
    #[error("ADS: License not limited in time (0x729)")]
    AdsErrDeviceLicenseNoTimeLimit,
    #[error("ADS: Licensing problem: time in the future (0x72A)")]
    AdsErrDeviceLicenseFutureIssue,
    #[error("ADS: License period too long (0x72B)")]
    AdsErrDeviceLicenseTimeTooLong,
    #[error("ADS: Exception at system startup (0x72C)")]
    AdsErrDeviceException,
    #[error("ADS: License file read twice (0x72D)")]
    AdsErrDeviceLicenseDuplicated,
    #[error("ADS: Invalid signature (0x72E)")]
    AdsErrDeviceSignatureInvalid,
    #[error("ADS: Invalid certificate (0x72F)")]
    AdsErrDeviceCertificateInvalid,
    #[error("ADS: Public key not known from OEM (0x730)")]
    AdsErrDeviceLicenseOemNotFound,
    #[error("ADS: License not valid for this system ID (0x731)")]
    AdsErrDeviceLicenseRestricted,
    #[error("ADS: Demo license prohibited (0x732)")]
    AdsErrDeviceLicenseDemoDenied,
    #[error("ADS: Invalid function ID (0x733)")]
    AdsErrDeviceInvalidFncId,
    #[error("ADS: Outside the valid range (0x734)")]
    AdsErrDeviceOutOfRange,
    #[error("ADS: Invalid alignment (0x735)")]
    AdsErrDeviceInvalidAlignment,
    #[error("ADS: Invalid platform level (0x736)")]
    AdsErrDeviceLicensePlatform,
    #[error("ADS: Context – forward to passive level (0x737)")]
    AdsErrDeviceForwardPl,
    #[error("ADS: Context – forward to dispatch level (0x738)")]
    AdsErrDeviceForwardDl,
    #[error("ADS: Context – forward to real-time (0x739)")]
    AdsErrDeviceForwardRt,

    // --- Client Errors (0x740 .. 0x756) ---
    #[error("Client: Client error (0x740)")]
    AdsErrClientError,
    #[error("Client: Service contains an invalid parameter (0x741)")]
    AdsErrClientInvalidParm,
    #[error("Client: Polling list is empty (0x742)")]
    AdsErrClientListEmpty,
    #[error("Client: Var connection already in use (0x743)")]
    AdsErrClientVarUsed,
    #[error("Client: The called ID is already in use (0x744)")]
    AdsErrClientDuplInvokeId,
    #[error(
        "Client: Timeout has occurred – the remote terminal is not responding in the specified ADS timeout. \
        The route setting of the remote terminal may be configured incorrectly (0x745)"
    )]
    AdsErrClientSyncTimeout,
    #[error("Client: Error in Win32 subsystem (0x746)")]
    AdsErrClientW32Error,
    #[error("Client: Invalid client timeout value (0x747)")]
    AdsErrClientTimeoutInvalid,
    #[error("Client: Port not open (0x748)")]
    AdsErrClientPortNotOpen,
    #[error("Client: No AMS address (0x749)")]
    AdsErrClientNoAmsAddr,
    #[error("Client: Internal error in Ads sync (0x750)")]
    AdsErrClientSyncInternal,
    #[error("Client: Hash table overflow (0x751)")]
    AdsErrClientAddHash,
    #[error("Client: Key not found in the table (0x752)")]
    AdsErrClientRemoveHash,
    #[error("Client: No symbols in the cache (0x753)")]
    AdsErrClientNoMoreSym,
    #[error("Client: Invalid response received (0x754)")]
    AdsErrClientSyncResInvalid,
    #[error("Client: Sync port is locked (0x755)")]
    AdsErrClientSyncPortLocked,
    #[error("Client: The request was canceled (0x756)")]
    AdsErrClientRequestCancelled,

    // --- RTime Errors (0x1000 .. 0x101A) ---
    #[error("RTime: Internal error in the real-time system (0x1000)")]
    RtErrInternal,
    #[error("RTime: Timer value is not valid (0x1001)")]
    RtErrBadTimerPeriods,
    #[error("RTime: Task pointer has the invalid value 0 (zero) (0x1002)")]
    RtErrInvalidTaskPtr,
    #[error("RTime: Stack pointer has the invalid value 0 (zero) (0x1003)")]
    RtErrInvalidStackPtr,
    #[error("RTime: The request task priority is already assigned (0x1004)")]
    RtErrPrioExists,
    #[error(
        "RTime: No free TCB (Task Control Block) available. The maximum number of TCBs is 64 (0x1005)"
    )]
    RtErrNoMoreTcb,
    #[error("RTime: No free semaphores available. The maximum number of semaphores is 64 (0x1006)")]
    RtErrNoMoreSemas,
    #[error(
        "RTime: No free space available in the queue. \
        The maximum number of positions in the queue is 64 (0x1007)"
    )]
    RtErrNoMoreQueues,
    #[error("RTime: An external synchronization interrupt is already applied (0x100D)")]
    RtErrExtIrqAlreadyDef,
    #[error("RTime: No external sync interrupt applied (0x100E)")]
    RtErrExtIrqNotDef,
    #[error("RTime: Application of the external synchronization interrupt has failed. (0x100F)")]
    RtErrExtIrqInstallFailed,
    #[error("RTime: Call of a service function in the wrong context (0x1010)")]
    RtErrIrqlNotLessOrEqual,
    #[error("RTime: Intel VT-x extension is not supported (0x1017)")]
    RtErrVmxNotSupported,
    #[error("RTime: Intel VT-x extension is not enabled in the BIOS (0x1018)")]
    RtErrVmxDisabled,
    #[error("RTime: Missing function in Intel VT-x extension (0x1019)")]
    RtErrVmxControlsMissing,
    #[error("RTime: Activation of Intel VT-x fails (0x101A)")]
    RtErrVmxEnableFails,

    // --- TCP Winsock Errors (Common ones) ---
    #[error(
        "Winsock: A connection timeout has occurred - error while establishing the connection, \
        because the remote terminal did not respond properly after a certain period of time, \
        or the established connection could not be maintained because \
        the connected host did not respond (0x274C)"
    )]
    WsaETimedOut,
    #[error(
        "Winsock: Connection refused - no connection could be established because the target computer \
        has explicitly rejected it. This error usually results from an attempt to connect to a service \
        that is inactive on the external host, that is, \
        a service for which no server application is running (0x274D)"
    )]
    WsaEConnRefused,
    #[error(
        "Winsock: No route to host - a socket operation referred to an unavailable host (0x2751)"
    )]
    WsaEHostUnreach,

    // --- Fallback ---
    #[error("Unknown ADS Return Code: {0:#X}")]
    Unknown(u32),
}

impl AdsReturnCode {
    /// The length of the ADS return code in bytes.
    pub const LENGTH: usize = 4;

    /// Returns `true` if the code represents success (0).
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Creates a new `AdsReturnCode` from a 4-byte array (Little Endian).
    pub fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self::from(bytes)
    }

    /// Converts the return code to a 4-byte array (Little Endian).
    pub fn to_bytes(&self) -> [u8; Self::LENGTH] {
        (*self).into()
    }

    /// Tries to create a new `AdsReturnCode` from a 4-byte array (Little Endian).
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, AdsReturnCodeError> {
        bytes.try_into()
    }
}

impl From<u32> for AdsReturnCode {
    fn from(code: u32) -> Self {
        match code {
            0x0 => Self::Ok,

            // --- Global Error Codes (0x01 .. 0x1E) ---
            0x01 => Self::ErrInternal,
            0x02 => Self::ErrNoRtime,
            0x03 => Self::ErrAllocLockedMem,
            0x04 => Self::ErrInsertMailbox,
            0x05 => Self::ErrWrongReceiveHMsg,
            0x06 => Self::ErrTargetPortNotFound,
            0x07 => Self::ErrTargetMachineNotFound,
            0x08 => Self::ErrUnknownCmdId,
            0x09 => Self::ErrBadTaskId,
            0x0A => Self::ErrNoIo,
            0x0B => Self::ErrUnknownAmsCmd,
            0x0C => Self::ErrWin32Error,
            0x0D => Self::ErrPortNotConnected,
            0x0E => Self::ErrInvalidAmsLength,
            0x0F => Self::ErrInvalidAmsNetId,
            0x10 => Self::ErrLowInstLevel,
            0x11 => Self::ErrNoDebug,
            0x12 => Self::ErrPortDisabled,
            0x13 => Self::ErrPortAlreadyConnected,
            0x14 => Self::ErrAmsSyncW32Error,
            0x15 => Self::ErrAmsSyncTimeout,
            0x16 => Self::ErrAmsSyncError,
            0x17 => Self::ErrAmsSyncNoIndexInMap,
            0x18 => Self::ErrInvalidAmsPort,
            0x19 => Self::ErrNoMemory,
            0x1A => Self::ErrTcpSend,
            0x1B => Self::ErrHostUnreachable,
            0x1C => Self::ErrInvalidAmsFragment,
            0x1D => Self::ErrTlsSend,
            0x1E => Self::ErrAccessDenied,

            // --- Router Error Codes (0x500 .. 0x50D) ---
            0x500 => Self::RouterErrNoLockedMemory,
            0x501 => Self::RouterErrResizeMemory,
            0x502 => Self::RouterErrMailboxFull,
            0x503 => Self::RouterErrDebugBoxFull,
            0x504 => Self::RouterErrUnknownPortType,
            0x505 => Self::RouterErrNotInitialized,
            0x506 => Self::RouterErrPortAlreadyInUse,
            0x507 => Self::RouterErrNotRegistered,
            0x508 => Self::RouterErrNoMoreQueues,
            0x509 => Self::RouterErrInvalidPort,
            0x50A => Self::RouterErrNotActivated,
            0x50B => Self::RouterErrFragmentBoxFull,
            0x50C => Self::RouterErrFragmentTimeout,
            0x50D => Self::RouterErrToBeRemoved,

            // --- General ADS Error Codes (0x700 .. 0x756) ---
            0x700 => Self::AdsErrDeviceError,
            0x701 => Self::AdsErrDeviceSrvNotSupp,
            0x702 => Self::AdsErrDeviceInvalidGrp,
            0x703 => Self::AdsErrDeviceInvalidOffset,
            0x704 => Self::AdsErrDeviceInvalidAccess,
            0x705 => Self::AdsErrDeviceInvalidSize,
            0x706 => Self::AdsErrDeviceInvalidData,
            0x707 => Self::AdsErrDeviceNotReady,
            0x708 => Self::AdsErrDeviceBusy,
            0x709 => Self::AdsErrDeviceInvalidContext,
            0x70A => Self::AdsErrDeviceNoMemory,
            0x70B => Self::AdsErrDeviceInvalidParm,
            0x70C => Self::AdsErrDeviceNotFound,
            0x70D => Self::AdsErrDeviceSyntax,
            0x70E => Self::AdsErrDeviceIncompatible,
            0x70F => Self::AdsErrDeviceExists,
            0x710 => Self::AdsErrDeviceSymbolNotFound,
            0x711 => Self::AdsErrDeviceSymbolVersionInvalid,
            0x712 => Self::AdsErrDeviceInvalidState,
            0x713 => Self::AdsErrDeviceTransModeNotSupp,
            0x714 => Self::AdsErrDeviceNotifyHndInvalid,
            0x715 => Self::AdsErrDeviceClientUnknown,
            0x716 => Self::AdsErrDeviceNoMoreHdls,
            0x717 => Self::AdsErrDeviceInvalidWatchSize,
            0x718 => Self::AdsErrDeviceNotInit,
            0x719 => Self::AdsErrDeviceTimeout,
            0x71A => Self::AdsErrDeviceNoInterface,
            0x71B => Self::AdsErrDeviceInvalidInterface,
            0x71C => Self::AdsErrDeviceInvalidClsId,
            0x71D => Self::AdsErrDeviceInvalidObjId,
            0x71E => Self::AdsErrDevicePending,
            0x71F => Self::AdsErrDeviceAborted,
            0x720 => Self::AdsErrDeviceWarning,
            0x721 => Self::AdsErrDeviceInvalidArrayIdx,
            0x722 => Self::AdsErrDeviceSymbolNotActive,
            0x723 => Self::AdsErrDeviceAccessDenied,
            0x724 => Self::AdsErrDeviceLicenseNotFound,
            0x725 => Self::AdsErrDeviceLicenseExpired,
            0x726 => Self::AdsErrDeviceLicenseExceeded,
            0x727 => Self::AdsErrDeviceLicenseInvalid,
            0x728 => Self::AdsErrDeviceLicenseSystemId,
            0x729 => Self::AdsErrDeviceLicenseNoTimeLimit,
            0x72A => Self::AdsErrDeviceLicenseFutureIssue,
            0x72B => Self::AdsErrDeviceLicenseTimeTooLong,
            0x72C => Self::AdsErrDeviceException,
            0x72D => Self::AdsErrDeviceLicenseDuplicated,
            0x72E => Self::AdsErrDeviceSignatureInvalid,
            0x72F => Self::AdsErrDeviceCertificateInvalid,
            0x730 => Self::AdsErrDeviceLicenseOemNotFound,
            0x731 => Self::AdsErrDeviceLicenseRestricted,
            0x732 => Self::AdsErrDeviceLicenseDemoDenied,
            0x733 => Self::AdsErrDeviceInvalidFncId,
            0x734 => Self::AdsErrDeviceOutOfRange,
            0x735 => Self::AdsErrDeviceInvalidAlignment,
            0x736 => Self::AdsErrDeviceLicensePlatform,
            0x737 => Self::AdsErrDeviceForwardPl,
            0x738 => Self::AdsErrDeviceForwardDl,
            0x739 => Self::AdsErrDeviceForwardRt,

            // --- Client Errors (0x740 .. 0x756) ---
            0x740 => Self::AdsErrClientError,
            0x741 => Self::AdsErrClientInvalidParm,
            0x742 => Self::AdsErrClientListEmpty,
            0x743 => Self::AdsErrClientVarUsed,
            0x744 => Self::AdsErrClientDuplInvokeId,
            0x745 => Self::AdsErrClientSyncTimeout,
            0x746 => Self::AdsErrClientW32Error,
            0x747 => Self::AdsErrClientTimeoutInvalid,
            0x748 => Self::AdsErrClientPortNotOpen,
            0x749 => Self::AdsErrClientNoAmsAddr,
            0x750 => Self::AdsErrClientSyncInternal,
            0x751 => Self::AdsErrClientAddHash,
            0x752 => Self::AdsErrClientRemoveHash,
            0x753 => Self::AdsErrClientNoMoreSym,
            0x754 => Self::AdsErrClientSyncResInvalid,
            0x755 => Self::AdsErrClientSyncPortLocked,
            0x756 => Self::AdsErrClientRequestCancelled,

            // --- RTime Errors (0x1000 .. 0x101A) ---
            0x1000 => Self::RtErrInternal,
            0x1001 => Self::RtErrBadTimerPeriods,
            0x1002 => Self::RtErrInvalidTaskPtr,
            0x1003 => Self::RtErrInvalidStackPtr,
            0x1004 => Self::RtErrPrioExists,
            0x1005 => Self::RtErrNoMoreTcb,
            0x1006 => Self::RtErrNoMoreSemas,
            0x1007 => Self::RtErrNoMoreQueues,
            0x100D => Self::RtErrExtIrqAlreadyDef,
            0x100E => Self::RtErrExtIrqNotDef,
            0x100F => Self::RtErrExtIrqInstallFailed,
            0x1010 => Self::RtErrIrqlNotLessOrEqual,
            0x1017 => Self::RtErrVmxNotSupported,
            0x1018 => Self::RtErrVmxDisabled,
            0x1019 => Self::RtErrVmxControlsMissing,
            0x101A => Self::RtErrVmxEnableFails,

            // --- TCP Winsock Errors ---
            10060 => Self::WsaETimedOut,
            10061 => Self::WsaEConnRefused,
            10065 => Self::WsaEHostUnreach,

            // --- Fallback ---
            n => Self::Unknown(n),
        }
    }
}

impl From<AdsReturnCode> for u32 {
    fn from(value: AdsReturnCode) -> Self {
        match value {
            AdsReturnCode::Ok => 0x0,

            // --- Global Error Codes (0x01 .. 0x1E) ---
            AdsReturnCode::ErrInternal => 0x01,
            AdsReturnCode::ErrNoRtime => 0x02,
            AdsReturnCode::ErrAllocLockedMem => 0x03,
            AdsReturnCode::ErrInsertMailbox => 0x04,
            AdsReturnCode::ErrWrongReceiveHMsg => 0x05,
            AdsReturnCode::ErrTargetPortNotFound => 0x06,
            AdsReturnCode::ErrTargetMachineNotFound => 0x07,
            AdsReturnCode::ErrUnknownCmdId => 0x08,
            AdsReturnCode::ErrBadTaskId => 0x09,
            AdsReturnCode::ErrNoIo => 0x0A,
            AdsReturnCode::ErrUnknownAmsCmd => 0x0B,
            AdsReturnCode::ErrWin32Error => 0x0C,
            AdsReturnCode::ErrPortNotConnected => 0x0D,
            AdsReturnCode::ErrInvalidAmsLength => 0x0E,
            AdsReturnCode::ErrInvalidAmsNetId => 0x0F,
            AdsReturnCode::ErrLowInstLevel => 0x10,
            AdsReturnCode::ErrNoDebug => 0x11,
            AdsReturnCode::ErrPortDisabled => 0x12,
            AdsReturnCode::ErrPortAlreadyConnected => 0x13,
            AdsReturnCode::ErrAmsSyncW32Error => 0x14,
            AdsReturnCode::ErrAmsSyncTimeout => 0x15,
            AdsReturnCode::ErrAmsSyncError => 0x16,
            AdsReturnCode::ErrAmsSyncNoIndexInMap => 0x17,
            AdsReturnCode::ErrInvalidAmsPort => 0x18,
            AdsReturnCode::ErrNoMemory => 0x19,
            AdsReturnCode::ErrTcpSend => 0x1A,
            AdsReturnCode::ErrHostUnreachable => 0x1B,
            AdsReturnCode::ErrInvalidAmsFragment => 0x1C,
            AdsReturnCode::ErrTlsSend => 0x1D,
            AdsReturnCode::ErrAccessDenied => 0x1E,

            // --- Router Error Codes (0x500 .. 0x50D) ---
            AdsReturnCode::RouterErrNoLockedMemory => 0x500,
            AdsReturnCode::RouterErrResizeMemory => 0x501,
            AdsReturnCode::RouterErrMailboxFull => 0x502,
            AdsReturnCode::RouterErrDebugBoxFull => 0x503,
            AdsReturnCode::RouterErrUnknownPortType => 0x504,
            AdsReturnCode::RouterErrNotInitialized => 0x505,
            AdsReturnCode::RouterErrPortAlreadyInUse => 0x506,
            AdsReturnCode::RouterErrNotRegistered => 0x507,
            AdsReturnCode::RouterErrNoMoreQueues => 0x508,
            AdsReturnCode::RouterErrInvalidPort => 0x509,
            AdsReturnCode::RouterErrNotActivated => 0x50A,
            AdsReturnCode::RouterErrFragmentBoxFull => 0x50B,
            AdsReturnCode::RouterErrFragmentTimeout => 0x50C,
            AdsReturnCode::RouterErrToBeRemoved => 0x50D,

            // --- General ADS Error Codes (0x700 .. 0x756) ---
            AdsReturnCode::AdsErrDeviceError => 0x700,
            AdsReturnCode::AdsErrDeviceSrvNotSupp => 0x701,
            AdsReturnCode::AdsErrDeviceInvalidGrp => 0x702,
            AdsReturnCode::AdsErrDeviceInvalidOffset => 0x703,
            AdsReturnCode::AdsErrDeviceInvalidAccess => 0x704,
            AdsReturnCode::AdsErrDeviceInvalidSize => 0x705,
            AdsReturnCode::AdsErrDeviceInvalidData => 0x706,
            AdsReturnCode::AdsErrDeviceNotReady => 0x707,
            AdsReturnCode::AdsErrDeviceBusy => 0x708,
            AdsReturnCode::AdsErrDeviceInvalidContext => 0x709,
            AdsReturnCode::AdsErrDeviceNoMemory => 0x70A,
            AdsReturnCode::AdsErrDeviceInvalidParm => 0x70B,
            AdsReturnCode::AdsErrDeviceNotFound => 0x70C,
            AdsReturnCode::AdsErrDeviceSyntax => 0x70D,
            AdsReturnCode::AdsErrDeviceIncompatible => 0x70E,
            AdsReturnCode::AdsErrDeviceExists => 0x70F,
            AdsReturnCode::AdsErrDeviceSymbolNotFound => 0x710,
            AdsReturnCode::AdsErrDeviceSymbolVersionInvalid => 0x711,
            AdsReturnCode::AdsErrDeviceInvalidState => 0x712,
            AdsReturnCode::AdsErrDeviceTransModeNotSupp => 0x713,
            AdsReturnCode::AdsErrDeviceNotifyHndInvalid => 0x714,
            AdsReturnCode::AdsErrDeviceClientUnknown => 0x715,
            AdsReturnCode::AdsErrDeviceNoMoreHdls => 0x716,
            AdsReturnCode::AdsErrDeviceInvalidWatchSize => 0x717,
            AdsReturnCode::AdsErrDeviceNotInit => 0x718,
            AdsReturnCode::AdsErrDeviceTimeout => 0x719,
            AdsReturnCode::AdsErrDeviceNoInterface => 0x71A,
            AdsReturnCode::AdsErrDeviceInvalidInterface => 0x71B,
            AdsReturnCode::AdsErrDeviceInvalidClsId => 0x71C,
            AdsReturnCode::AdsErrDeviceInvalidObjId => 0x71D,
            AdsReturnCode::AdsErrDevicePending => 0x71E,
            AdsReturnCode::AdsErrDeviceAborted => 0x71F,
            AdsReturnCode::AdsErrDeviceWarning => 0x720,
            AdsReturnCode::AdsErrDeviceInvalidArrayIdx => 0x721,
            AdsReturnCode::AdsErrDeviceSymbolNotActive => 0x722,
            AdsReturnCode::AdsErrDeviceAccessDenied => 0x723,
            AdsReturnCode::AdsErrDeviceLicenseNotFound => 0x724,
            AdsReturnCode::AdsErrDeviceLicenseExpired => 0x725,
            AdsReturnCode::AdsErrDeviceLicenseExceeded => 0x726,
            AdsReturnCode::AdsErrDeviceLicenseInvalid => 0x727,
            AdsReturnCode::AdsErrDeviceLicenseSystemId => 0x728,
            AdsReturnCode::AdsErrDeviceLicenseNoTimeLimit => 0x729,
            AdsReturnCode::AdsErrDeviceLicenseFutureIssue => 0x72A,
            AdsReturnCode::AdsErrDeviceLicenseTimeTooLong => 0x72B,
            AdsReturnCode::AdsErrDeviceException => 0x72C,
            AdsReturnCode::AdsErrDeviceLicenseDuplicated => 0x72D,
            AdsReturnCode::AdsErrDeviceSignatureInvalid => 0x72E,
            AdsReturnCode::AdsErrDeviceCertificateInvalid => 0x72F,
            AdsReturnCode::AdsErrDeviceLicenseOemNotFound => 0x730,
            AdsReturnCode::AdsErrDeviceLicenseRestricted => 0x731,
            AdsReturnCode::AdsErrDeviceLicenseDemoDenied => 0x732,
            AdsReturnCode::AdsErrDeviceInvalidFncId => 0x733,
            AdsReturnCode::AdsErrDeviceOutOfRange => 0x734,
            AdsReturnCode::AdsErrDeviceInvalidAlignment => 0x735,
            AdsReturnCode::AdsErrDeviceLicensePlatform => 0x736,
            AdsReturnCode::AdsErrDeviceForwardPl => 0x737,
            AdsReturnCode::AdsErrDeviceForwardDl => 0x738,
            AdsReturnCode::AdsErrDeviceForwardRt => 0x739,

            // --- Client Errors (0x740 .. 0x756) ---
            AdsReturnCode::AdsErrClientError => 0x740,
            AdsReturnCode::AdsErrClientInvalidParm => 0x741,
            AdsReturnCode::AdsErrClientListEmpty => 0x742,
            AdsReturnCode::AdsErrClientVarUsed => 0x743,
            AdsReturnCode::AdsErrClientDuplInvokeId => 0x744,
            AdsReturnCode::AdsErrClientSyncTimeout => 0x745,
            AdsReturnCode::AdsErrClientW32Error => 0x746,
            AdsReturnCode::AdsErrClientTimeoutInvalid => 0x747,
            AdsReturnCode::AdsErrClientPortNotOpen => 0x748,
            AdsReturnCode::AdsErrClientNoAmsAddr => 0x749,
            AdsReturnCode::AdsErrClientSyncInternal => 0x750,
            AdsReturnCode::AdsErrClientAddHash => 0x751,
            AdsReturnCode::AdsErrClientRemoveHash => 0x752,
            AdsReturnCode::AdsErrClientNoMoreSym => 0x753,
            AdsReturnCode::AdsErrClientSyncResInvalid => 0x754,
            AdsReturnCode::AdsErrClientSyncPortLocked => 0x755,
            AdsReturnCode::AdsErrClientRequestCancelled => 0x756,

            // --- RTime Errors (0x1000 .. 0x101A) ---
            AdsReturnCode::RtErrInternal => 0x1000,
            AdsReturnCode::RtErrBadTimerPeriods => 0x1001,
            AdsReturnCode::RtErrInvalidTaskPtr => 0x1002,
            AdsReturnCode::RtErrInvalidStackPtr => 0x1003,
            AdsReturnCode::RtErrPrioExists => 0x1004,
            AdsReturnCode::RtErrNoMoreTcb => 0x1005,
            AdsReturnCode::RtErrNoMoreSemas => 0x1006,
            AdsReturnCode::RtErrNoMoreQueues => 0x1007,
            AdsReturnCode::RtErrExtIrqAlreadyDef => 0x100D,
            AdsReturnCode::RtErrExtIrqNotDef => 0x100E,
            AdsReturnCode::RtErrExtIrqInstallFailed => 0x100F,
            AdsReturnCode::RtErrIrqlNotLessOrEqual => 0x1010,
            AdsReturnCode::RtErrVmxNotSupported => 0x1017,
            AdsReturnCode::RtErrVmxDisabled => 0x1018,
            AdsReturnCode::RtErrVmxControlsMissing => 0x1019,
            AdsReturnCode::RtErrVmxEnableFails => 0x101A,

            // --- TCP Winsock Errors ---
            AdsReturnCode::WsaETimedOut => 10060,
            AdsReturnCode::WsaEConnRefused => 10061,
            AdsReturnCode::WsaEHostUnreach => 10065,

            // --- Fallback ---
            AdsReturnCode::Unknown(n) => n,
        }
    }
}

impl From<[u8; Self::LENGTH]> for AdsReturnCode {
    fn from(value: [u8; Self::LENGTH]) -> Self {
        u32::from_le_bytes(value).into()
    }
}

impl From<AdsReturnCode> for [u8; AdsReturnCode::LENGTH] {
    fn from(value: AdsReturnCode) -> Self {
        u32::from(value).to_le_bytes()
    }
}

impl TryFrom<&[u8]> for AdsReturnCode {
    type Error = AdsReturnCodeError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < AdsReturnCode::LENGTH {
            return Err(AdsReturnCodeError::UnexpectedLength {
                expected: AdsReturnCode::LENGTH,
                got: value.len(),
            });
        }
        let arr = [value[0], value[1], value[2], value[3]];
        Ok(Self::from(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u32() {
        assert_eq!(AdsReturnCode::from(0x0), AdsReturnCode::Ok);
    }

    #[test]
    fn test_from_ads_return_code() {
        assert_eq!(AdsReturnCode::from(AdsReturnCode::Ok), AdsReturnCode::Ok);
    }

    #[test]
    fn test_from_ads_return_code_to_u32() {
        assert_eq!(u32::from(AdsReturnCode::RtErrIrqlNotLessOrEqual), 0x1010);
    }

    #[test]
    fn test_from_ads_return_code_to_bytes() {
        let bytes: [u8; AdsReturnCode::LENGTH] = AdsReturnCode::RtErrIrqlNotLessOrEqual.into();
        assert_eq!([0x10, 0x10, 0x00, 0x00], bytes);
    }
}
