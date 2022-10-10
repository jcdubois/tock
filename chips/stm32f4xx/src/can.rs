#[warn(unused_attributes)]
use core::cell::Cell;
use kernel::debug;
use kernel::platform::chip::ClockInterface;
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable};
use kernel::utilities::registers::{register_bitfields, register_structs, ReadWrite};
use kernel::utilities::StaticRef;
use kernel::hil::can;
use crate::rcc;

#[repr(C)]
struct TransmitMailBox {
    can_tir: ReadWrite<u32, CAN_TIxR::Register>,
    can_tdtr: ReadWrite<u32, CAN_TDTxR::Register>,
    can_tdlr: ReadWrite<u32, CAN_TDLxR::Register>,
    can_tdhr: ReadWrite<u32, CAN_TDHxR::Register>,
}
struct ReceiveMailBox {
    _can_rir: ReadWrite<u32, CAN_RIxR::Register>,
    _can_rdtr: ReadWrite<u32, CAN_RDTxR::Register>,
    can_rdlr: ReadWrite<u32, CAN_RDLxR::Register>,
    can_rdhr: ReadWrite<u32, CAN_RDHxR::Register>,
}

register_structs! {
    pub Registers {
        /// CAN control and status registers
        (0x000 => can_mcr: ReadWrite<u32, CAN_MCR::Register>),
        /// CAN master status register
        (0x004 => can_msr: ReadWrite<u32, CAN_MSR::Register>),
        /// CAN transmit status register
        (0x008 => can_tsr: ReadWrite<u32, CAN_TSR::Register>),
        /// CAN receive FIFO 0 register
        (0x00c => can_rf0r: ReadWrite<u32, CAN_RF0R::Register>),
        /// CAN receive FIFO 1 registers
        (0x010 => can_rf1r: ReadWrite<u32, CAN_RF1R::Register>),
        /// CAN interrupt enable register
        (0x014 => can_ier: ReadWrite<u32, CAN_IER::Register>),
        /// CAN error status register
        (0x018 => can_esr: ReadWrite<u32, CAN_ESR::Register>),
        /// CAN bit timing register
        (0x01c => can_btr: ReadWrite<u32, CAN_BTR::Register>),
        (0x020 => _reserved0),
        ///
        ///
        /// CAN MAILBOX REGISTERS
        ///
        /// CAN TX mailbox identifier registers
        (0x180 => can_tx_mailbox: [TransmitMailBox; 3]),
        /// CAN RX mailbox identifier registers
        (0x1b0 => can_rx_mailbox: [ReceiveMailBox; 2]),
        (0x1d0 => _reserved1),
        ///
        ///
        /// CAN FILTER REGISTERS
        ///
        ///
        /// CAN filter master register
        (0x200 => can_fmr: ReadWrite<u32, CAN_FMR::Register>),
        /// CAN filter mode register
        (0x204 => can_fm1r: ReadWrite<u32, CAN_FM1R::Register>),
        (0x208 => _reserved2),
        /// CAN filter scale register
        (0x20c => can_fs1r: ReadWrite<u32, CAN_FS1R::Register>),
        (0x210 => _reserved3),
        /// CAN filter FIFO assignment register
        (0x214 => can_ffa1r: ReadWrite<u32, CAN_FFA1R::Register>),
        (0x218 => _reserved4),
        /// CAN filter activation register
        (0x21c => can_fa1r: ReadWrite<u32, CAN_FA1R::Register>),
        (0x220 => _reserved5),
        /// Filter bank 0-27 for register 1-2
        (0x240 => can_firx: [ReadWrite<u32, CAN_FiRx::Register>; 56]),
        (0x320 => @END),
    }
}

register_bitfields![u32,
    CAN_MCR [
        /// Debug freeze
        DBF OFFSET(16) NUMBITS(1) [],
        /// bcXAN software master reset
        RESET OFFSET(15) NUMBITS(1) [],
        /// Time triggered communication mode
        TTCM OFFSET(7) NUMBITS(1) [],
        /// Automatic bus-off management
        ABOM OFFSET(6) NUMBITS(1) [],
        /// Automatic wakeup mode
        AWUM OFFSET(5) NUMBITS(1) [],
        /// No automatic retransmission
        NART OFFSET(4) NUMBITS(1) [],
        /// Receive FIFO locked mode
        RFLM OFFSET(3) NUMBITS(1) [],
        /// Transmit FIFO prioritY
        TXFP OFFSET(2) NUMBITS(1) [],
        /// Sleep mode request
        SLEEP OFFSET(1) NUMBITS(1) [],
        /// Initialization request
        INRQ OFFSET(0) NUMBITS(1) []
    ],
    CAN_MSR [
        /// CAN Rx signal
        RX OFFSET(11) NUMBITS(1) [],
        /// Last sample point
        SAMP OFFSET(10) NUMBITS(1) [],
        /// Receive mode
        RXM OFFSET(9) NUMBITS(1) [],
        /// Transmit mode
        TXM OFFSET(8) NUMBITS(1) [],
        /// Sleep acknowledge interrupt
        SLAKI OFFSET(4) NUMBITS(1) [],
        /// Wakeup interrupt
        WKUI OFFSET(3) NUMBITS(1) [],
        /// Error interrupt
        ERRI OFFSET(2) NUMBITS(1) [],
        /// Sleep acknowledge
        SLAK OFFSET(1) NUMBITS(1) [],
        /// Initialization acknowledge
        INAK OFFSET(0) NUMBITS(1) []
    ],
    CAN_TSR [
        /// Lowest priority flag for mailbox 2
        LOW2 OFFSET(31) NUMBITS(1) [],
        /// Lowest priority flag for mailbox 1
        LOW1 OFFSET(30) NUMBITS(1) [],
        /// Lowest priority flag for mailbox 0
        LOW0 OFFSET(29) NUMBITS(1) [],
        /// Transmit mailbox 2 empty
        TME2 OFFSET(28) NUMBITS(1) [],
        /// Transmit mailbox 1 empty
        TME1 OFFSET(27) NUMBITS(1) [],
        /// Transmit mailbox 0 empty
        TME0 OFFSET(26) NUMBITS(1) [],
        /// Mailbox code
        CODE OFFSET(24) NUMBITS(2) [],
        /// Abort request for mailbox 2
        ABRQ2 OFFSET(23) NUMBITS(1) [],
        /// Transmission error of mailbox 2
        TERR2 OFFSET(19) NUMBITS(1) [],
        /// Arbitration lost for mailbox 2
        ALST2 OFFSET(18) NUMBITS(1) [],
        /// Transmission OK of mailbox 2
        TXOK2 OFFSET(17) NUMBITS(1) [],
        /// Request completed mailbox 2
        RQCP2 OFFSET(16) NUMBITS(1) [],
        /// Abort request for mailbox 1
        ABRQ1 OFFSET(15) NUMBITS(1) [],
        /// Transmission error of mailbox 1
        TERR1 OFFSET(11) NUMBITS(1) [],
        /// Arbitration lost for mailbox 1
        ALST1 OFFSET(10) NUMBITS(1) [],
        /// Transmission OK of mailbox 1
        TXOK1 OFFSET(9) NUMBITS(1) [],
        /// Request completed mailbox 1
        RQCP1 OFFSET(8) NUMBITS(1) [],
        /// Abort request for mailbox 0
        ABRQ0 OFFSET(7) NUMBITS(1) [],
        /// Transmission error of mailbox 0
        TERR0 OFFSET(3) NUMBITS(1) [],
        /// Arbitration lost for mailbox 0
        ALST0 OFFSET(2) NUMBITS(1) [],
        /// Transmission OK of mailbox 0
        TXOK0 OFFSET(1) NUMBITS(1) [],
        /// Request completed mailbox 0
        RQCP0 OFFSET(0) NUMBITS(1) []
    ],
    CAN_RF0R [
        /// Release FIFO 0 output mailbox
        RFOM0 OFFSET(5) NUMBITS(1) [],
        /// FIFO 0 overrun
        FOVR0 OFFSET(4) NUMBITS(1) [],
        /// FIFO 0 full
        FULL0 OFFSET(3) NUMBITS(1) [],
        /// FIFO 0 message pending
        FMP0 OFFSET(0) NUMBITS(2) []
    ],
    CAN_RF1R [
        /// Release FIFO 1 output mailbox
        RFOM1 OFFSET(5) NUMBITS(1) [],
        /// FIFO 1 overrun
        FOVR1 OFFSET(4) NUMBITS(1) [],
        /// FIFO 1 full
        FULL1 OFFSET(3) NUMBITS(1) [],
        /// FIFO 1 message pending
        FMP1 OFFSET(0) NUMBITS(2) []
    ],
    CAN_IER [
        /// Sleep interrupt enable
        SLKIE OFFSET(17) NUMBITS(1) [],
        /// Wakeup interrupt enable
        WKUIE OFFSET(16) NUMBITS(1) [],
        /// Error interrupt enable
        ERRIE OFFSET(15) NUMBITS(1) [],
        /// Last error code interrupt enable
        LECIE OFFSET(11) NUMBITS(1) [],
        /// Bus-off interrupt enable
        BOFIE OFFSET(10) NUMBITS(1) [],
        /// Error passive interrupt enable
        EPVIE OFFSET(9) NUMBITS(1) [],
        /// Error warning interrupt enable
        EWGIE OFFSET(8) NUMBITS(1) [],
        /// FIFO 1 overrun interrupt enable
        FOVIE1 OFFSET(6) NUMBITS(1) [],
        /// FIFO 1 full interrupt enable
        FFIE1 OFFSET(5) NUMBITS(1) [],
        /// FIFO 1 message pending interrupt enable
        FMPIE1 OFFSET(4) NUMBITS(1) [],
        /// FIFO 0 overrun interrupt enable
        FOVIE0 OFFSET(3) NUMBITS(1) [],
        /// FIFO 0 full interrupt enable
        FFIE0 OFFSET(2) NUMBITS(1) [],
        /// FIFO 0 message pending interrupt enable
        FMPIE0 OFFSET(1) NUMBITS(1) [],
        /// Transmit mailbox empty interrupt enable
        TMEIE OFFSET(0) NUMBITS(1) []
    ],
    CAN_ESR [
        /// Receive error counter
        REC OFFSET(24) NUMBITS(8) [],
        /// Least significant byte of the 9-bit transmit error counter
        TEC OFFSET(16) NUMBITS(8) [],
        /// Last error code
        LEC OFFSET(4) NUMBITS(3) [
            NoError = 0,
            StuffError = 1,
            FormError = 2,
            AcknowledgmentError = 3,
            BitRecessiveError = 4,
            BitDominantError = 5,
            CrcError = 6,
            SetBySoftware = 7
        ],
        /// Bus-off flag
        BOFF OFFSET(2) NUMBITS(1) [],
        /// Error passive flag
        EPVF OFFSET(1) NUMBITS(1) [],
        /// Error warning flag
        EWGF OFFSET(0) NUMBITS(1) []
    ],
    CAN_BTR [
        /// Silent mode (debug)
        SILM OFFSET(31) NUMBITS(1) [],
        /// Loop back mode (debug)
        LBKM OFFSET(30) NUMBITS(1) [],
        /// Resynchronization jump width
        SJW OFFSET(24) NUMBITS(2) [],
        /// Time segment 2
        TS2 OFFSET(20) NUMBITS(3) [],
        /// Time segment 1
        TS1 OFFSET(16) NUMBITS(4) [],
        /// Baud rate prescaler
        BRP OFFSET(0) NUMBITS(10) []
    ],
    ///
    ///
    /// CAN mailbox registers
    ///
    ///
    CAN_TIxR [
        /// Standard identifier or extended identifier
        STID OFFSET(21) NUMBITS(11) [],
        /// Extended identifier
        EXID OFFSET(3) NUMBITS(18) [],
        /// Identifier extension
        IDE OFFSET(2) NUMBITS(1) [],
        /// Remote transmission request
        RTR OFFSET(1) NUMBITS(1) [],
        /// Transmit mailbox request
        TXRQ OFFSET(0) NUMBITS(1) []
    ],
    CAN_TDTxR [
        /// Message time stamp
        TIME OFFSET(16) NUMBITS(16) [],
        /// Transmit global time
        TGT OFFSET(8) NUMBITS(1) [],
        /// Data length code
        DLC OFFSET(0) NUMBITS(4) []
    ],
    CAN_TDLxR [
        /// Data byte 3
        DATA3 OFFSET(24) NUMBITS(8) [],
        /// Data byte 2
        DATA2 OFFSET(16) NUMBITS(8) [],
        /// Data byte 1
        DATA1 OFFSET(8) NUMBITS(8) [],
        /// Data byte 0
        DATA0 OFFSET(0) NUMBITS(8) []
    ],
    CAN_TDHxR [
        /// Data byte 7
        DATA7 OFFSET(24) NUMBITS(8) [],
        /// Data byte 6
        DATA6 OFFSET(16) NUMBITS(8) [],
        /// Data byte 5
        DATA5 OFFSET(8) NUMBITS(8) [],
        /// Data byte 4
        DATA4 OFFSET(0) NUMBITS(8) []
    ],
    CAN_RIxR [
        /// Standard identifier or extended identifier
        STID OFFSET(21) NUMBITS(11) [],
        /// Extended identifier
        EXID OFFSET(3) NUMBITS(18) [],
        /// Identifier extension
        IDE OFFSET(2) NUMBITS(1) [],
        /// Remote transmission request
        RTR OFFSET(1) NUMBITS(1) []
    ],
    CAN_RDTxR [
        /// Message time stamp
        TIME OFFSET(16) NUMBITS(16) [],
        /// Filter match index
        FMI OFFSET(8) NUMBITS(8) [],
        /// Data length code
        DLC OFFSET(0) NUMBITS(4) []
    ],
    CAN_RDLxR [
        /// Data byte 3
        DATA3 OFFSET(24) NUMBITS(8) [],
        /// Data byte 2
        DATA2 OFFSET(16) NUMBITS(8) [],
        /// Data byte 1
        DATA1 OFFSET(8) NUMBITS(8) [],
        /// Data byte 0
        DATA0 OFFSET(0) NUMBITS(8) []
    ],
    CAN_RDHxR [
        /// Data byte 7
        DATA7 OFFSET(24) NUMBITS(8) [],
        /// Data byte 6
        DATA6 OFFSET(16) NUMBITS(8) [],
        /// Data byte 5
        DATA5 OFFSET(8) NUMBITS(8) [],
        /// Data byte 4
        DATA4 OFFSET(0) NUMBITS(8) []
    ],
    ///
    ///
    /// CAN filter registers
    ///
    ///
    CAN_FMR [
        /// CAN start bank
        CANSB OFFSET(8) NUMBITS(6) [],
        /// Filter initialization mode
        FINIT OFFSET(0) NUMBITS(1) []
    ],
    /// CAN filter mode register
    CAN_FM1R [
        /// Filter mode
        FBM OFFSET(0) NUMBITS(28) []
    ],
    CAN_FS1R [
        /// Filter scale configuration
        FSC OFFSET(0) NUMBITS(28) []
    ],
    CAN_FFA1R [
        /// Filter FIFO assignment for filter x
        FFA OFFSET(0) NUMBITS(28) []
    ],
    CAN_FA1R [
        /// Filter active
        FACT OFFSET(0) NUMBITS(28) []
    ],
    CAN_FiRx [
        /// Filter bits
        FB OFFSET(0) NUMBITS(32) []
    ]
];

#[derive(Copy, Clone, PartialEq)]
enum CanState {
    Initialization,
    Normal,
    Sleep,
}

#[allow(dead_code)]
#[repr(u32)]
enum BitSegment1 {
    CanBtrTs1_1tq = 0b0000,
    CanBtrTs1_2tq = 0b0001,
    CanBtrTs1_3tq = 0b0010,
    CanBtrTs1_4tq = 0b0011,
    CanBtrTs1_5tq = 0b0100,
    CanBtrTs1_6tq = 0b0101,
    CanBtrTs1_7tq = 0b0110,
    CanBtrTs1_8tq = 0b0111,
    CanBtrTs1_9tq = 0b1000,
    CanBtrTs1_10tq = 0b1001,
    CanBtrTs1_11tq = 0b1010,
    CanBtrTs1_12tq = 0b1011,
    CanBtrTs1_13tq = 0b1100,
    CanBtrTs1_14tq = 0b1101,
    CanBtrTs1_15tq = 0b1110,
    CanBtrTs1_16tq = 0b1111,
}

#[allow(dead_code)]
#[repr(u32)]
enum BitSegment2 {
    CanBtrTs2_1tq = 0b0000,
    CanBtrTs2_2tq = 0b0001,
    CanBtrTs2_3tq = 0b0010,
    CanBtrTs2_4tq = 0b0011,
    CanBtrTs2_5tq = 0b0100,
    CanBtrTs2_6tq = 0b0101,
    CanBtrTs2_7tq = 0b0110,
    CanBtrTs2_8tq = 0b0111,
}

#[allow(dead_code)]
#[repr(u32)]
enum SynchronizationJumpWidth {
    CanBtrSjw1tq = 0b00,
    CanBtrSjw2tq = 0b01,
    CanBtrSjw3tq = 0b10,
    CanBtrSjw4tq = 0b11,
}

#[derive(Copy, Clone, PartialEq)]
pub enum CanInterruptMode {
    TransmitInterrupt,
    Fifo0Interrupt,
    Fifo1Interrupt,
    ErrorAndStatusChangeInterrupt,
}

impl From<CanState> for can::State {
    fn from(state: CanState) -> Self {
        match state {
            CanState::Initialization | CanState::Sleep => can::State::Disabled,
            CanState::Normal => can::State::Running,
        }
    }
}

pub struct Can<'a> {
    registers: StaticRef<Registers>,
    clock: CanClock<'a>,
    can_state: Cell<CanState>,
    error_interrupt_counter: Cell<u32>,
    fifo0_interrupt_counter: Cell<u32>,
    fifo1_interrupt_counter: Cell<u32>,
    check: Cell<u32>,
    failed_messages: Cell<u32>,
    automatic_retransmission: Cell<bool>,
    automatic_wake_up: Cell<bool>,
    operating_mode: OptionalCell<can::OperationMode>,
    bit_timing: OptionalCell<can::BitTiming>,
    controller_client: OptionalCell<&'static dyn can::ControllerClient>,
    receive_client: OptionalCell<&'static dyn can::ReceiveClient>,
    transmit_client: OptionalCell<&'static dyn can::TransmitClient<{ can::STANDARD_CAN_PACKET_SIZE }>>,
    rx_buffer: TakeCell<'static, [u8]>,
    tx_buffer: TakeCell<'static, [u8; can::STANDARD_CAN_PACKET_SIZE]>,
}

impl<'a> Can<'a> {
    pub fn new(rcc: &'a rcc::Rcc, registers: StaticRef<Registers>) -> Can<'a> {
        Can {
            registers: registers,
            clock: CanClock(rcc::PeripheralClock::new(
                rcc::PeripheralClockType::APB1(rcc::PCLK1::CAN1),
                rcc,
            )),
            can_state: Cell::new(CanState::Sleep),
            error_interrupt_counter: Cell::new(0),
            fifo0_interrupt_counter: Cell::new(0),
            fifo1_interrupt_counter: Cell::new(0),
            check: Cell::new(10),
            failed_messages: Cell::new(0),
            automatic_retransmission: Cell::new(false),
            automatic_wake_up: Cell::new(false),
            operating_mode: OptionalCell::empty(),
            bit_timing: OptionalCell::empty(),
            controller_client: OptionalCell::empty(),
            receive_client: OptionalCell::empty(),
            transmit_client: OptionalCell::empty(),
            rx_buffer: TakeCell::empty(),
            tx_buffer: TakeCell::empty(),
        }
    }

    fn wait_for(times: usize, f: impl Fn() -> bool) -> Result<(), kernel::ErrorCode> {
        for _ in 0..times {
            if f() {
                return Ok(());
            }
        }

        Err(kernel::ErrorCode::FAIL)
    }

    pub fn enable(&self) -> Result<(), kernel::ErrorCode> {
        // debug!("[enable]");
        // leave Sleep Mode
        self.registers.can_mcr.modify(CAN_MCR::SLEEP::CLEAR);

        // request to enter the initialization mode
        self.registers.can_mcr.modify(CAN_MCR::INRQ::SET);

        // we wait for hardware ACK (INAK bit to be set)
        if let Err(inak_err) = Can::wait_for(20000, || self.registers.can_msr.is_set(CAN_MSR::INAK))
        {
            return Err(inak_err);
        }

        self.can_state.set(CanState::Initialization);

        // we wait for hardware ACK (SLAK bit to be set)
        if let Err(slak_err) =
            Can::wait_for(20000, || !self.registers.can_msr.is_set(CAN_MSR::SLAK))
        {
            return Err(slak_err);
        }

        // set communication mode -- hardcoded for now
        self.registers.can_mcr.modify(CAN_MCR::TTCM::CLEAR);
        self.registers.can_mcr.modify(CAN_MCR::ABOM::CLEAR);
        self.registers.can_mcr.modify(CAN_MCR::RFLM::CLEAR);
        self.registers.can_mcr.modify(CAN_MCR::TXFP::CLEAR);

        match self.automatic_retransmission.get() {
            true => self.registers.can_mcr.modify(CAN_MCR::AWUM::SET),
            false => self.registers.can_mcr.modify(CAN_MCR::AWUM::CLEAR),
        }

        match self.automatic_wake_up.get() {
            true => self.registers.can_mcr.modify(CAN_MCR::NART::CLEAR),
            false => self.registers.can_mcr.modify(CAN_MCR::NART::SET),
        }

        // enter loopback mode - for debug
        // if let Some(operating_mode_settings) = self.operating_mode.extract() {
        //     match operating_mode_settings {
        //         can::OperationMode::Loopback => self.registers.can_btr.modify(CAN_BTR::LBKM::SET),
        //         can::OperationMode::Monitoring => self.registers.can_btr.modify(CAN_BTR::SILM::SET),
        //         can::OperationMode::Freeze => todo!(),
        //         can::OperationMode::Normal => todo!(),
        //     }
        // }
        

        // set bit timing mode - hardcoded for now
        if let Some(bit_timing_settings) = self.bit_timing.extract() {
            self.registers
                .can_btr
                .modify(CAN_BTR::TS1.val(bit_timing_settings.segment1 as u32));
            self.registers
                .can_btr
                .modify(CAN_BTR::TS2.val(bit_timing_settings.segment2 as u32));
            self.registers
                .can_btr
                .modify(CAN_BTR::SJW.val(bit_timing_settings.sync_jump_width as u32));
            self.registers
                .can_btr
                .modify(CAN_BTR::BRP.val(bit_timing_settings.baud_rate_prescaler as u32));
        } else {
            self.enter_sleep_mode();
            return Err(kernel::ErrorCode::INVAL);
        }

        Ok(())
    }

    pub fn config_filter(&self, filter_info: can::FilterParameters, enable: bool) {
        // get position of the filter number
        let filter_number = 0x00000001 << filter_info.number;

        // start filter configuration
        self.registers.can_fmr.modify(CAN_FMR::FINIT::SET);

        // request filter number filter_number
        self.registers.can_fa1r.modify(
            CAN_FA1R::FACT.val(self.registers.can_fa1r.read(CAN_FA1R::FACT) & !filter_number),
        );

        // request filter width to be 32 or 16 bits
        match filter_info.scale_bits {
            can::ScaleBits::Bits16 => {
                self.registers.can_fs1r.modify(
                    CAN_FS1R::FSC.val(self.registers.can_fs1r.read(CAN_FS1R::FSC) | filter_number),
                );
            }
            can::ScaleBits::Bits32 => {
                self.registers.can_fs1r.modify(
                    CAN_FS1R::FSC.val(self.registers.can_fs1r.read(CAN_FS1R::FSC) & !filter_number),
                );
            }
        }

        self.registers.can_firx[(filter_info.number as usize) * 2].modify(CAN_FiRx::FB.val(0));
        self.registers.can_firx[(filter_info.number as usize) * 2 + 1].modify(CAN_FiRx::FB.val(0));

        // request filter mode to be mask or list
        match filter_info.identifier_mode {
            can::IdentifierMode::List => {
                self.registers.can_fm1r.modify(
                    CAN_FM1R::FBM.val(self.registers.can_fm1r.read(CAN_FM1R::FBM) | filter_number),
                );
            }
            can::IdentifierMode::Mask => {
                self.registers.can_fm1r.modify(
                    CAN_FM1R::FBM.val(self.registers.can_fm1r.read(CAN_FM1R::FBM) & !filter_number),
                );
            }
        }

        // request fifo0 or fifo1
        if filter_info.fifo_number == 0 {
            self.registers.can_ffa1r.modify(
                CAN_FFA1R::FFA.val(self.registers.can_ffa1r.read(CAN_FFA1R::FFA) & !filter_number),
            );
        } else {
            self.registers.can_ffa1r.modify(
                CAN_FFA1R::FFA.val(self.registers.can_ffa1r.read(CAN_FFA1R::FFA) | filter_number),
            );
        }

        if enable {
            self.registers.can_fa1r.modify(
                CAN_FA1R::FACT.val(self.registers.can_fa1r.read(CAN_FA1R::FACT) | filter_number),
            );
        } else {
            self.registers.can_fa1r.modify(
                CAN_FA1R::FACT.val(self.registers.can_fa1r.read(CAN_FA1R::FACT) & !filter_number),
            );
        }
    }

    pub fn enable_filter_config(&self) {
        // activate the filter configuration
        self.registers.can_fmr.modify(CAN_FMR::FINIT::CLEAR);
    }

    pub fn enter_normal_mode(&self) -> Result<(), kernel::ErrorCode> {
        // debug!("[enter_normal_mode]");
        // request to enter normal mode by clearing INRQ bit
        self.registers.can_mcr.modify(CAN_MCR::INRQ::CLEAR);
        // // wait for INAK bit to be cleared
        // panic!("a mers pana aici\n");
        if let Err(inak_err) = Can::wait_for(20000, || !self.registers.can_msr.is_set(CAN_MSR::INAK)) {
            return Err(inak_err);
        }

        debug!("INAK {}", self.registers.can_msr.is_set(CAN_MSR::INAK));

        if self.registers.can_msr.is_set(CAN_MSR::INAK) {
            self.check.replace(100);
        }

        self.can_state.set(CanState::Normal);
        Ok(())

        // debug!("[enter normal mode] can_btr este {:x}", self.registers.can_btr.get());

        // for i in 0..5 {
        //     // debug! ("[enter normal mode] este a {} tura - urmeaza sa trimitem mesaj", i);
        //     let data: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        //     self.send_8byte_message(true, 0x334455, 8, 0, data);
        // }

        // debug!("[enter normal mode] avem {} mesage netrimise\n", self.failed_messages.get());
    }

    pub fn enter_sleep_mode(&self) {
        // request to enter sleep mode by setting SLEEP bit
        self.registers.can_mcr.modify(CAN_MCR::SLEEP::SET);
        self.can_state.set(CanState::Sleep);
    }


    pub fn send_8byte_message(
        &self,
        id: can::Id,
        dlc: usize,
        rtr: u8,
    ) -> Result<(), kernel::ErrorCode> {
        self.enable_irq(CanInterruptMode::ErrorAndStatusChangeInterrupt);
        if self.can_state.get() == CanState::Normal {
            if let Some(tx_mailbox) = self.find_empty_mailbox() {
                // debug!("[send_8byte_message] mailbox {} and {:?}", tx_mailbox, data);
                // set extended or standard id in registers
                match id {
                    can::Id::Standard(id) => {
                        // debug!("[start transmission] normal id\n");
                        self.registers.can_tx_mailbox[tx_mailbox]
                            .can_tir
                            .modify(CAN_TIxR::IDE::CLEAR);
                        self.registers.can_tx_mailbox[tx_mailbox]
                            .can_tir
                            .modify(CAN_TIxR::STID.val(id as u32 & 0xeff));
                        self.registers.can_tx_mailbox[tx_mailbox]
                            .can_tir
                            .modify(CAN_TIxR::EXID.val(0));
                    }
                    can::Id::Extended(id) => {
                        // debug!("[start transmission] extended id\n");
                        self.registers.can_tx_mailbox[tx_mailbox]
                            .can_tir
                            .modify(CAN_TIxR::IDE::SET);
                        self.registers.can_tx_mailbox[tx_mailbox]
                            .can_tir
                            .modify(CAN_TIxR::STID.val((id & 0xffc0000) >> 18));
                        self.registers.can_tx_mailbox[tx_mailbox]
                            .can_tir
                            .modify(CAN_TIxR::EXID.val(id & 0x003fffff));
                    }
                }
                // write rtr
                self.registers.can_tx_mailbox[tx_mailbox]
                    .can_tir
                    .modify(CAN_TIxR::RTR.val(rtr.into()));
                // write dlc
                self.registers.can_tx_mailbox[tx_mailbox]
                    .can_tdtr
                    .modify(CAN_TDTxR::DLC.val(dlc as u32));
                // write first 4 bytes of the data
                // debug!("[start transmission] write first 4 bytes of data\n");
                match self.tx_buffer.map(|tx| {
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdlr
                        .modify(CAN_TDLxR::DATA0.val(tx[0].into()));
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdlr
                        .modify(CAN_TDLxR::DATA1.val(tx[1].into()));
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdlr
                        .modify(CAN_TDLxR::DATA2.val(tx[2].into()));
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdlr
                        .modify(CAN_TDLxR::DATA3.val(tx[3].into()));
                    // write the last 4 bytes of the data
                    // debug!("[start transmission] write last 4 bytes of data\n");
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdhr
                        .modify(CAN_TDHxR::DATA4.val(tx[4].into()));
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdhr
                        .modify(CAN_TDHxR::DATA5.val(tx[5].into()));
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdhr
                        .modify(CAN_TDHxR::DATA6.val(tx[6].into()));
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tdhr
                        .modify(CAN_TDHxR::DATA7.val(tx[7].into()));
                    
                    self.registers.can_tx_mailbox[tx_mailbox]
                        .can_tir
                        .modify(CAN_TIxR::TXRQ::SET);
                    
                }) {
                    Some(_) => Ok(()),
                    None => Err(kernel::ErrorCode::FAIL),
                }                
            } else {
                self.failed_messages.replace(self.failed_messages.get() + 1);
                Err(kernel::ErrorCode::BUSY)
                // no mailbox empty
            }
        } else {
            Err(kernel::ErrorCode::OFF)
        }
    }

    pub fn find_empty_mailbox(&self) -> Option<usize> {
        // let res = self.mailbox_counter.get();
        // self.mailbox_counter.replace((res + 1) % 3);
        // Some(res as usize)
        if self.registers.can_tsr.read(CAN_TSR::TME0) == 1 {
            Some(0)
        } else if self.registers.can_tsr.read(CAN_TSR::TME1) == 1 {
            Some(1)
        } else if self.registers.can_tsr.read(CAN_TSR::TME2) == 1 {
            Some(2)
        } else {
            None
        }
    }

    pub fn is_enabled_clock(&self) -> bool {
        self.clock.is_enabled()
    }

    pub fn enable_clock(&self) {
        self.clock.enable();
    }

    pub fn disable_clock(&self) {
        self.clock.disable();
    }

    pub fn send_transmit_callback(&self) {
       
    }
    pub fn handle_transmit_interrupt(&self) {
        // debug!("[handle tx interrupt] transmit_interrupt_handler");
        // check the TX fifo where the interrupt was triggered
        // let mut send_callback = false;
        let mailbox0_status: u32 = self.registers.can_tsr.read(CAN_TSR::RQCP0);
        if mailbox0_status == 1 {
            // check status
            let transmit_status: u32 = self.registers.can_tsr.read(CAN_TSR::TXOK0);
            if transmit_status == 1 {
                // mark the interrupt as handled
                self.registers.can_tsr.modify(CAN_TSR::RQCP0::SET);
            }
        }
        let mailbox1_status: u32 = self.registers.can_tsr.read(CAN_TSR::RQCP1);
        if mailbox1_status == 1 {
            let transmit_status: u32 = self.registers.can_tsr.read(CAN_TSR::TXOK1);
            if transmit_status == 1 {
                // mark the interrupt as handled
                self.registers.can_tsr.modify(CAN_TSR::RQCP1::SET);
            }
        }
        let mailbox2_status: u32 = self.registers.can_tsr.read(CAN_TSR::RQCP2);
        if mailbox2_status == 1 {
            let transmit_status: u32 = self.registers.can_tsr.read(CAN_TSR::TXOK2);
            if transmit_status == 1 {
                // mark the interrupt as handled
                self.registers.can_tsr.modify(CAN_TSR::RQCP2::SET);
            }
        }
        
        self.transmit_client.map(|transmit_client| {
            match self.tx_buffer.take() {
                Some(buf) => {
                    transmit_client.transmit_complete(Ok(()), buf)
                }
                None => {},
            }
        });    
    }

    pub fn convert_u32_to_arr(&self, input1: u32, input2: u32) -> [u8; 8] {
        let b1: u8 = ((input1 >> 24) & 0xff) as u8;
        let b2: u8 = ((input1 >> 16) & 0xff) as u8;
        let b3: u8 = ((input1 >> 8) & 0xff) as u8;
        let b4: u8 = (input1 & 0xff) as u8;
        let b5: u8 = ((input2 >> 24) & 0xff) as u8;
        let b6: u8 = ((input2 >> 16) & 0xff) as u8;
        let b7: u8 = ((input2 >> 8) & 0xff) as u8;
        let b8: u8 = (input2 & 0xff) as u8;
        // todo test the right order here
        [b4, b3, b2, b1, b8, b7, b6, b5]
    }

    pub fn handle_fifo0_interrupt(&self) {
        // debug!("[handle rx0 interrupt] fifo0_interrupt_handler");
        let new_message_reception_status: u32 = self.registers.can_rf0r.read(CAN_RF0R::FMP0);
        let full_condition_status: u32 = self.registers.can_rf0r.read(CAN_RF0R::FULL0);
        let overrun_condition_status: u32 = self.registers.can_rf0r.read(CAN_RF0R::FOVR0);

        if full_condition_status == 1 {
            // debug!("[handle rx0 interrupt] received full fifo interrupt");
            self.registers.can_rf0r.modify(CAN_RF0R::FULL0::SET);
        }

        if overrun_condition_status == 1 {
            // debug!("[handle rx0 interrupt] received overrun fifo interrupt");
            self.registers.can_rf0r.modify(CAN_RF0R::FOVR0::SET);
        }

        if new_message_reception_status != 0 {
            let message_id = if self.registers.can_rx_mailbox[0]
                ._can_rir
                .read(CAN_RIxR::IDE)
                == 0
            {
                can::Id::Standard(
                    self.registers.can_rx_mailbox[0]
                        ._can_rir
                        .read(CAN_RIxR::STID) as u16,
                )
            } else {
                can::Id::Extended(
                    (self.registers.can_rx_mailbox[0]
                        ._can_rir
                        .read(CAN_RIxR::STID)
                        << 18)
                        | (self.registers.can_rx_mailbox[0]
                            ._can_rir
                            .read(CAN_RIxR::EXID)),
                )
            };
            let message_length = self.registers.can_rx_mailbox[0]
                ._can_rdtr
                .read(CAN_RDTxR::DLC) as usize;
            let mut rx_buf = self.convert_u32_to_arr(
                self.registers.can_rx_mailbox[0].can_rdlr.get(),
                self.registers.can_rx_mailbox[0].can_rdhr.get(),
            );
            self.rx_buffer.map(|rx| {
                // todo change
                for i in 0..8 {
                    rx[i] = rx_buf[i];
                }
            });
            self.receive_client.map(|receive_client| {
                receive_client.message_received(message_id, rx_buf.as_mut(), message_length, Ok(()))
            });
            self.fifo0_interrupt_counter
                .replace(self.fifo0_interrupt_counter.get() + 1);
            if self.fifo0_interrupt_counter.get() % 1000 == 0 {
                debug!(
                    "[handle rx0 interrupt] we received {} messages on fifo0",
                    self.fifo0_interrupt_counter.get()
                );
            }
            // mark the interrupt as handled
            self.registers.can_rf0r.modify(CAN_RF0R::RFOM0::SET);
        }

        // if self.check.get() != 0 {
        //     if self.check.get() == 100 {
        //         debug!("[handle rx0 interrupt] check este 100, initial era true inak");
        //     }
        //     let data: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        //     debug!("[handle rx0 interrupt] urmeaza sa trimitem mesaj");
        //     self.send_8byte_message(true, 0x334455, 8, 0, data);
        //     self.check.replace(0);
        // }
    }

    pub fn handle_fifo1_interrupt(&self) {
        // debug!("[handle rx1 interrupt] fifo1_interrupt_handler");
        let new_message_reception_status: u32 = self.registers.can_rf1r.read(CAN_RF1R::FMP1);
        let full_condition_status: u32 = self.registers.can_rf1r.read(CAN_RF1R::FULL1);
        let overrun_condition_status: u32 = self.registers.can_rf1r.read(CAN_RF1R::FOVR1);

        if full_condition_status == 1 {
            // debug!("[handle rx1 interrupt] received full fifo interrupt");
            self.registers.can_rf1r.modify(CAN_RF1R::FULL1::SET);
        }

        if overrun_condition_status == 1 {
            // debug!("[handle rx1 interrupt] received overrun fifo interrupt");
            self.registers.can_rf1r.modify(CAN_RF1R::FOVR1::SET);
        }

        if new_message_reception_status != 0 {
            self.fifo1_interrupt_counter
                .replace(self.fifo1_interrupt_counter.get() + 1);
            let message_id = if self.registers.can_rx_mailbox[1]
                ._can_rir
                .read(CAN_RIxR::IDE)
                == 0
            {
                can::Id::Standard(
                    self.registers.can_rx_mailbox[1]
                        ._can_rir
                        .read(CAN_RIxR::STID) as u16,
                )
            } else {
                can::Id::Extended(
                    (self.registers.can_rx_mailbox[1]
                        ._can_rir
                        .read(CAN_RIxR::STID)
                        << 18)
                        | (self.registers.can_rx_mailbox[1]
                            ._can_rir
                            .read(CAN_RIxR::EXID)),
                )
            };
            let message_length = self.registers.can_rx_mailbox[1]
                ._can_rdtr
                .read(CAN_RDTxR::DLC) as usize;
            let mut rx_buf = self.convert_u32_to_arr(
                self.registers.can_rx_mailbox[1].can_rdlr.get(),
                self.registers.can_rx_mailbox[1].can_rdhr.get(),
            );
            self.rx_buffer.map(|rx| {
                // todo change
                for i in 0..8 {
                    rx[i] = rx_buf[i];
                }
            });
            self.receive_client.map(|receive_client| {
                receive_client.message_received(message_id, rx_buf.as_mut(), message_length, Ok(()))
            });
            // mark the interrupt as handled
            self.registers.can_rf1r.modify(CAN_RF1R::RFOM1::SET);
        }
    }

    pub fn handle_error_status_interrupt(&self) {
        debug!("[handle error/status change interrupt]");
        if self.registers.can_esr.read(CAN_ESR::EWGF) == 1 {
            debug!("[handle error/status change interrupt] error warning flag");
        }
        if self.registers.can_esr.read(CAN_ESR::EPVF) == 1 {
            debug!("[handle error/status change interrupt] error passive flag");
        }
        if self.registers.can_esr.read(CAN_ESR::BOFF) == 1 {
            debug!("[handle error/status change interrupt] bus off error");
        }
        if self.registers.can_esr.read(CAN_ESR::LEC) != 0 {
            debug!(
                "[handle error/status change interrupt] last error code: {}",
                self.registers.can_esr.read(CAN_ESR::LEC)
            );
        }
        if self.registers.can_msr.read(CAN_MSR::WKUI) == 1 {
            debug!(
                "[handle error/status change interrupt] wakeup interrupt error, inak este {}",
                self.registers.can_msr.is_set(CAN_MSR::INAK)
            );
            self.registers.can_msr.modify(CAN_MSR::WKUI::SET);
        }
        if self.registers.can_msr.read(CAN_MSR::SLAK) == 1 {
            debug!("[handle error/status change interrupt] sleep ack error");
        }
        self.error_interrupt_counter
            .replace(self.error_interrupt_counter.get() + 1);
        if self.error_interrupt_counter.get() > 10 {
            self.disable_irq(CanInterruptMode::ErrorAndStatusChangeInterrupt);
            // debug!("error_and_status_change interrupt\n");
            // debug!(
            //     "avem arbitration lost for mailbox0: {}",
            //     self.registers.can_tsr.read(CAN_TSR::ALST0)
            // );
            // debug!(
            //     "avem transmission err for mailbox0: {}",
            //     self.registers.can_tsr.read(CAN_TSR::TERR0)
            // );
        }
    }

    pub fn enable_irq(&self, interrupt: CanInterruptMode) {
        match interrupt {
            CanInterruptMode::TransmitInterrupt => {
                self.registers.can_ier.modify(CAN_IER::TMEIE::SET);
            }
            CanInterruptMode::Fifo0Interrupt => {
                self.registers.can_ier.modify(CAN_IER::FMPIE0::SET);
                self.registers.can_ier.modify(CAN_IER::FFIE0::SET);
                self.registers.can_ier.modify(CAN_IER::FOVIE0::SET);
            }
            CanInterruptMode::Fifo1Interrupt => {
                self.registers.can_ier.modify(CAN_IER::FMPIE1::SET);
                self.registers.can_ier.modify(CAN_IER::FFIE1::SET);
                self.registers.can_ier.modify(CAN_IER::FOVIE1::SET);
            }
            CanInterruptMode::ErrorAndStatusChangeInterrupt => {
                self.registers.can_ier.modify(CAN_IER::ERRIE::SET);
                self.registers.can_ier.modify(CAN_IER::EWGIE::SET);
                self.registers.can_ier.modify(CAN_IER::EPVIE::SET);
                self.registers.can_ier.modify(CAN_IER::BOFIE::SET);
                self.registers.can_ier.modify(CAN_IER::LECIE::SET);
                self.registers.can_ier.modify(CAN_IER::WKUIE::SET);
                self.registers.can_ier.modify(CAN_IER::SLKIE::SET);
            }
        }
    }

    pub fn disable_irq(&self, interrupt: CanInterruptMode) {
        match interrupt {
            CanInterruptMode::TransmitInterrupt => {
                self.registers.can_ier.modify(CAN_IER::TMEIE::CLEAR);
            }
            CanInterruptMode::Fifo0Interrupt => {
                self.registers.can_ier.modify(CAN_IER::FMPIE0::CLEAR);
                self.registers.can_ier.modify(CAN_IER::FFIE0::CLEAR);
                self.registers.can_ier.modify(CAN_IER::FOVIE0::CLEAR);
            }
            CanInterruptMode::Fifo1Interrupt => {
                self.registers.can_ier.modify(CAN_IER::FMPIE1::CLEAR);
                self.registers.can_ier.modify(CAN_IER::FFIE1::CLEAR);
                self.registers.can_ier.modify(CAN_IER::FOVIE1::CLEAR);
            }
            CanInterruptMode::ErrorAndStatusChangeInterrupt => {
                self.registers.can_ier.modify(CAN_IER::ERRIE::CLEAR);
                self.registers.can_ier.modify(CAN_IER::EWGIE::CLEAR);
                self.registers.can_ier.modify(CAN_IER::EPVIE::CLEAR);
                self.registers.can_ier.modify(CAN_IER::BOFIE::CLEAR);
                self.registers.can_ier.modify(CAN_IER::LECIE::CLEAR);
                self.registers.can_ier.modify(CAN_IER::WKUIE::CLEAR);
                self.registers.can_ier.modify(CAN_IER::SLKIE::CLEAR);
            }
        }
    }

    pub fn enable_irqs(&self) {
        self.enable_irq(CanInterruptMode::TransmitInterrupt);
        self.enable_irq(CanInterruptMode::Fifo0Interrupt);
        self.enable_irq(CanInterruptMode::Fifo1Interrupt);
        self.enable_irq(CanInterruptMode::ErrorAndStatusChangeInterrupt);
    }

    pub fn disable_irqs(&self) {
        self.disable_irq(CanInterruptMode::TransmitInterrupt);
        self.disable_irq(CanInterruptMode::Fifo0Interrupt);
        self.disable_irq(CanInterruptMode::Fifo1Interrupt);
        self.disable_irq(CanInterruptMode::ErrorAndStatusChangeInterrupt);
    }
}

struct CanClock<'a>(rcc::PeripheralClock<'a>);

impl ClockInterface for CanClock<'_> {
    fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    fn enable(&self) {
        self.0.enable();
    }

    fn disable(&self) {
        self.0.disable();
    }
}

impl<'a> can::Configure for Can<'_> {
    fn set_bit_timing(&self, bit_timing: can::BitTiming) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Sleep => {
                self.bit_timing.set(bit_timing);
                Ok(())
            }
            CanState::Normal | CanState::Initialization => Err(kernel::ErrorCode::BUSY),
        }
    }

    fn set_operation_mode(&self, mode: can::OperationMode) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Sleep => {
                self.operating_mode.set(mode);
                Ok(())
            }
            CanState::Normal | CanState::Initialization => Err(kernel::ErrorCode::BUSY),
        }
    }

    fn get_bit_timing(&self) -> Result<can::BitTiming, kernel::ErrorCode> {
        if let Some(bit_timing) = self.bit_timing.extract() {
            Ok(bit_timing)
        } else {
            Err(kernel::ErrorCode::INVAL)
        }
    }

    fn get_operation_mode(&self) -> Result<can::OperationMode, kernel::ErrorCode> {
        if let Some(operation_mode) = self.operating_mode.extract() {
            Ok(operation_mode)
        } else {
            Err(kernel::ErrorCode::INVAL)
        }
    }

    fn set_automatic_retransmission(&self, automatic: bool) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Sleep => {
                self.automatic_retransmission.replace(automatic);
                Ok(())
            }
            CanState::Normal | CanState::Initialization => Err(kernel::ErrorCode::BUSY),
        }
    }

    fn set_wake_up(&self, wake_up: bool) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Sleep => {
                self.automatic_wake_up.replace(wake_up);
                Ok(())
            }
            CanState::Normal | CanState::Initialization => Err(kernel::ErrorCode::BUSY),
        }
    }

    fn get_automatic_retransmission(&self) -> Result<bool, kernel::ErrorCode> {
        Ok(self.automatic_retransmission.get())
    }

    fn get_wake_up(&self) -> Result<bool, kernel::ErrorCode> {
        Ok(self.automatic_wake_up.get())
    }

    fn receive_fifo_count(&self) -> usize {
        2
    }
}

impl<'a> can::Controller for Can<'_> {
    fn set_client(&self, client: Option<&'static dyn can::ControllerClient>) {
        if let Some(client) = client {
            self.controller_client.set(client);
        } else {
            self.controller_client.clear();
        }
    }

    fn enable(&self) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Sleep => {
                if self.bit_timing.is_none() || self.operating_mode.is_none() {
                    Err(kernel::ErrorCode::INVAL)
                } else {
                    if let Err(err) = self.enable() {
                        self.controller_client.map(|controller_client| {
                            controller_client.state_changed(self.can_state.get().into());
                            controller_client.enabled(Err(err));
                        });
                        Err(err)
                    } else {
                        if let Err(enable_err) = self.enter_normal_mode() {
                            return Err(enable_err);
                        };
                        self.controller_client.map(|controller_client| {
                            controller_client.state_changed(can::State::Running);
                            controller_client.enabled(Ok(can::State::Running));
                        });
                        Ok(())
                    }
                }
            }
            CanState::Normal | CanState::Initialization => Err(kernel::ErrorCode::BUSY),
        }
    }

    fn disable(&self) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Normal => {
                self.enter_sleep_mode();
                self.controller_client.map(|controller_client| {
                    controller_client.state_changed(self.can_state.get().into());
                    controller_client.disabled(Ok(()));
                });
                Ok(())
            }
            CanState::Sleep | CanState::Initialization => Err(kernel::ErrorCode::OFF),
        }
    }

    fn get_state(&self) -> Result<can::State, kernel::ErrorCode> {
        Ok(self.can_state.get().into())
    }
}

impl<'a> can::Transmit<{ can::STANDARD_CAN_PACKET_SIZE }> for Can<'_> {
    fn set_client(&self, client: Option<&'static dyn can::TransmitClient<{ can::STANDARD_CAN_PACKET_SIZE }>>) {
        if let Some(client) = client {
            self.transmit_client.set(client);
        } else {
            self.transmit_client.clear();
        }
    }

    fn send(
        &self,
        id: can::Id,
        buffer: &'static mut [u8; can::STANDARD_CAN_PACKET_SIZE],
        len: usize,
    ) -> Result<(), (kernel::ErrorCode, &'static mut [u8; can::STANDARD_CAN_PACKET_SIZE])> {
        debug!("INAK send {}", self.registers.can_msr.is_set(CAN_MSR::INAK));
        match self.can_state.get() {
            CanState::Normal => {
                self.tx_buffer.replace(buffer);
                self.enable_irq(CanInterruptMode::TransmitInterrupt);
                match self.send_8byte_message(id, len, 0) {
                    Ok(_) => Ok(()),
                    Err(err) => Err((err, self.tx_buffer.take().unwrap())),
                }
            }
            CanState::Sleep | CanState::Initialization => Err((kernel::ErrorCode::OFF, buffer)),
        }
    }
}

impl<'a> can::Receive for Can<'_> {
    fn set_client(&self, client: Option<&'static dyn can::ReceiveClient>) {
        if let Some(client) = client {
            self.receive_client.set(client);
        } else {
            self.receive_client.clear();
        }
    }

    fn start_receive_process(
        &self,
        buffer: &'static mut [u8],
    ) -> Result<(), (kernel::ErrorCode, &'static mut [u8])> {
        debug!("INAK receive {}", self.registers.can_msr.is_set(CAN_MSR::INAK));
        match self.can_state.get() {
            CanState::Normal => {
                self.config_filter(
                    can::FilterParameters {
                        number: 0,
                        scale_bits: can::ScaleBits::Bits32,
                        identifier_mode: can::IdentifierMode::Mask,
                        fifo_number: 0,
                    },
                    true,
                );
                self.config_filter(
                    can::FilterParameters {
                        number: 1,
                        scale_bits: can::ScaleBits::Bits32,
                        identifier_mode: can::IdentifierMode::Mask,
                        fifo_number: 1,
                    },
                    true,
                );
                self.enable_filter_config();
                self.enable_irq(CanInterruptMode::Fifo0Interrupt);
                self.enable_irq(CanInterruptMode::Fifo1Interrupt);
                self.rx_buffer.put(Some(buffer));
                Ok(())
            }
            CanState::Sleep | CanState::Initialization => Err((kernel::ErrorCode::OFF, buffer)),
        }
    }

    fn stop_receive(&self) -> Result<(), kernel::ErrorCode> {
        match self.can_state.get() {
            CanState::Normal => {
                self.config_filter(
                    can::FilterParameters {
                        number: 0,
                        scale_bits: can::ScaleBits::Bits32,
                        identifier_mode: can::IdentifierMode::Mask,
                        fifo_number: 0,
                    },
                    false,
                );
                self.config_filter(
                    can::FilterParameters {
                        number: 1,
                        scale_bits: can::ScaleBits::Bits32,
                        identifier_mode: can::IdentifierMode::Mask,
                        fifo_number: 1,
                    },
                    false,
                );
                self.enable_filter_config();
                self.disable_irq(CanInterruptMode::Fifo0Interrupt);
                self.disable_irq(CanInterruptMode::Fifo1Interrupt);
                match self.rx_buffer.take() {
                    Some(rx) => {
                        self.receive_client
                            .map(|receive_client| receive_client.stopped(rx));
                    }
                    None => {
                        return Err(kernel::ErrorCode::FAIL);
                    }
                }
                Ok(())
            }
            CanState::Sleep | CanState::Initialization => Err(kernel::ErrorCode::OFF),
        }
    }
}

// impl can::Filter for Can<'_> {
//     fn enable_filter(&self, _filter: can::FilterParameters) -> Result<(), kernel::ErrorCode> {
//         Ok(())
//     }

//     fn disable_filter(&self, _number: u32) -> Result<(), kernel::ErrorCode> {
//         Ok(())
//     }

//     fn filter_count(&self) -> usize {
//         14
//     }
// }