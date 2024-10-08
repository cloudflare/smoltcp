use core::fmt;

use byteorder::{ByteOrder, LittleEndian};

use crate::wire::ipv6::Address as Ipv6Address;
use crate::Error;
use crate::Result;

const CRC_TABLE: [u16; 256] = [
    0x0000, 0x1189, 0x2312, 0x329b, 0x4624, 0x57ad, 0x6536, 0x74bf, 0x8c48, 0x9dc1, 0xaf5a, 0xbed3,
    0xca6c, 0xdbe5, 0xe97e, 0xf8f7, 0x1081, 0x0108, 0x3393, 0x221a, 0x56a5, 0x472c, 0x75b7, 0x643e,
    0x9cc9, 0x8d40, 0xbfdb, 0xae52, 0xdaed, 0xcb64, 0xf9ff, 0xe876, 0x2102, 0x308b, 0x0210, 0x1399,
    0x6726, 0x76af, 0x4434, 0x55bd, 0xad4a, 0xbcc3, 0x8e58, 0x9fd1, 0xeb6e, 0xfae7, 0xc87c, 0xd9f5,
    0x3183, 0x200a, 0x1291, 0x0318, 0x77a7, 0x662e, 0x54b5, 0x453c, 0xbdcb, 0xac42, 0x9ed9, 0x8f50,
    0xfbef, 0xea66, 0xd8fd, 0xc974, 0x4204, 0x538d, 0x6116, 0x709f, 0x0420, 0x15a9, 0x2732, 0x36bb,
    0xce4c, 0xdfc5, 0xed5e, 0xfcd7, 0x8868, 0x99e1, 0xab7a, 0xbaf3, 0x5285, 0x430c, 0x7197, 0x601e,
    0x14a1, 0x0528, 0x37b3, 0x263a, 0xdecd, 0xcf44, 0xfddf, 0xec56, 0x98e9, 0x8960, 0xbbfb, 0xaa72,
    0x6306, 0x728f, 0x4014, 0x519d, 0x2522, 0x34ab, 0x0630, 0x17b9, 0xef4e, 0xfec7, 0xcc5c, 0xddd5,
    0xa96a, 0xb8e3, 0x8a78, 0x9bf1, 0x7387, 0x620e, 0x5095, 0x411c, 0x35a3, 0x242a, 0x16b1, 0x0738,
    0xffcf, 0xee46, 0xdcdd, 0xcd54, 0xb9eb, 0xa862, 0x9af9, 0x8b70, 0x8408, 0x9581, 0xa71a, 0xb693,
    0xc22c, 0xd3a5, 0xe13e, 0xf0b7, 0x0840, 0x19c9, 0x2b52, 0x3adb, 0x4e64, 0x5fed, 0x6d76, 0x7cff,
    0x9489, 0x8500, 0xb79b, 0xa612, 0xd2ad, 0xc324, 0xf1bf, 0xe036, 0x18c1, 0x0948, 0x3bd3, 0x2a5a,
    0x5ee5, 0x4f6c, 0x7df7, 0x6c7e, 0xa50a, 0xb483, 0x8618, 0x9791, 0xe32e, 0xf2a7, 0xc03c, 0xd1b5,
    0x2942, 0x38cb, 0x0a50, 0x1bd9, 0x6f66, 0x7eef, 0x4c74, 0x5dfd, 0xb58b, 0xa402, 0x9699, 0x8710,
    0xf3af, 0xe226, 0xd0bd, 0xc134, 0x39c3, 0x284a, 0x1ad1, 0x0b58, 0x7fe7, 0x6e6e, 0x5cf5, 0x4d7c,
    0xc60c, 0xd785, 0xe51e, 0xf497, 0x8028, 0x91a1, 0xa33a, 0xb2b3, 0x4a44, 0x5bcd, 0x6956, 0x78df,
    0x0c60, 0x1de9, 0x2f72, 0x3efb, 0xd68d, 0xc704, 0xf59f, 0xe416, 0x90a9, 0x8120, 0xb3bb, 0xa232,
    0x5ac5, 0x4b4c, 0x79d7, 0x685e, 0x1ce1, 0x0d68, 0x3ff3, 0x2e7a, 0xe70e, 0xf687, 0xc41c, 0xd595,
    0xa12a, 0xb0a3, 0x8238, 0x93b1, 0x6b46, 0x7acf, 0x4854, 0x59dd, 0x2d62, 0x3ceb, 0x0e70, 0x1ff9,
    0xf78f, 0xe606, 0xd49d, 0xc514, 0xb1ab, 0xa022, 0x92b9, 0x8330, 0x7bc7, 0x6a4e, 0x58d5, 0x495c,
    0x3de3, 0x2c6a, 0x1ef1, 0x0f78,
];

pub fn calculate_crc(buffer: &[u8]) -> u16 {
    fn crc_byte(crc: u16, c: u8) -> u16 {
        (crc >> 8) ^ CRC_TABLE[((crc ^ (c as u16)) & 0xff) as usize]
    }

    let mut crc = 0;

    for b in buffer {
        crc = crc_byte(crc, *b);
    }

    crc
}

enum_with_unknown! {
    /// IEEE 802.15.4 frame type.
    pub enum FrameType(u8) {
        Beacon = 0b000,
        Data = 0b001,
        Acknowledgement = 0b010,
        MacCommand = 0b011,
        Multipurpose = 0b101,
        FragmentOrFrak = 0b110,
        Extended = 0b111,
    }
}

impl fmt::Display for FrameType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrameType::Beacon => write!(f, "Beacon"),
            FrameType::Data => write!(f, "Data"),
            FrameType::Acknowledgement => write!(f, "Ack"),
            FrameType::MacCommand => write!(f, "MAC command"),
            FrameType::Multipurpose => write!(f, "Multipurpose"),
            FrameType::FragmentOrFrak => write!(f, "FragmentOrFrak"),
            FrameType::Extended => write!(f, "Extended"),
            FrameType::Unknown(id) => write!(f, "0b{:04b}", id),
        }
    }
}
enum_with_unknown! {
    /// IEEE 802.15.4 addressing mode for destination and source addresses.
    pub enum AddressingMode(u8) {
        Absent    = 0b00,
        Short     = 0b10,
        Extended  = 0b11,
    }
}

impl AddressingMode {
    /// Return the size in octets of the address.
    fn size(&self) -> usize {
        match self {
            AddressingMode::Absent => 0,
            AddressingMode::Short => 2,
            AddressingMode::Extended => 8,
            AddressingMode::Unknown(_) => 0, // TODO(thvdveld): what do we need to here?
        }
    }
}

impl fmt::Display for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddressingMode::Absent => write!(f, "Absent"),
            AddressingMode::Short => write!(f, "Short"),
            AddressingMode::Extended => write!(f, "Extended"),
            AddressingMode::Unknown(id) => write!(f, "0b{:04b}", id),
        }
    }
}

/// A IEEE 802.15.4 PAN.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Pan(pub u16);

impl Pan {
    pub const BROADCAST: Self = Self(0xffff);

    /// Return the PAN ID as bytes.
    pub fn as_bytes(&self) -> [u8; 2] {
        let mut pan = [0u8; 2];
        LittleEndian::write_u16(&mut pan, self.0);
        pan
    }
}

/// A IEEE 802.15.4 address.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Address {
    Absent,
    Short([u8; 2]),
    Extended([u8; 8]),
}

impl Address {
    /// The broadcast address.
    pub const BROADCAST: Address = Address::Short([0xff; 2]);

    /// Query whether the address is an unicast address.
    pub fn is_unicast(&self) -> bool {
        !self.is_broadcast()
    }

    /// Query whether this address is the broadcast address.
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }

    fn short_from_bytes(a: [u8; 2]) -> Self {
        Self::Short(a)
    }

    fn extended_from_bytes(a: [u8; 8]) -> Self {
        Self::Extended(a)
    }

    pub fn from_bytes(a: &[u8]) -> Self {
        if a.len() == 2 {
            let mut b = [0u8; 2];
            b.copy_from_slice(a);
            Address::Short(b)
        } else if a.len() == 8 {
            let mut b = [0u8; 8];
            b.copy_from_slice(a);
            Address::Extended(b)
        } else {
            panic!("Not an IEEE802.15.4 address");
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Address::Absent => &[],
            Address::Short(value) => value,
            Address::Extended(value) => value,
        }
    }

    /// Convert the extended address to an Extended Unique Identifier (EUI-64)
    pub fn as_eui_64(&self) -> Option<[u8; 8]> {
        match self {
            Address::Absent | Address::Short(_) => None,
            Address::Extended(value) => {
                let mut bytes = [0; 8];
                bytes.copy_from_slice(&value[..]);

                bytes[0] ^= 1 << 1;

                Some(bytes)
            }
        }
    }

    /// Convert an extended address to a link-local IPv6 address using the EUI-64 format from
    /// RFC2464.
    pub fn as_link_local_address(&self) -> Option<Ipv6Address> {
        let mut bytes = [0; 16];
        bytes[0] = 0xfe;
        bytes[1] = 0x80;
        bytes[8..].copy_from_slice(&self.as_eui_64()?);

        Some(Ipv6Address::from_bytes(&bytes))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Absent => write!(f, "not-present"),
            Self::Short(bytes) => write!(f, "{:02x}-{:02x}", bytes[0], bytes[1]),
            Self::Extended(bytes) => write!(
                f,
                "{:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}",
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
            ),
        }
    }
}

enum_with_unknown! {
    /// IEEE 802.15.4 addressing mode for destination and source addresses.
    pub enum FrameVersion(u8) {
        Ieee802154_2003 = 0b00,
        Ieee802154_2006 = 0b01,
        Ieee802154 = 0b10,
    }
}

/// A read/write wrapper around an IEEE 802.15.4 frame buffer.
#[derive(Debug, Clone)]
pub struct Frame<T: AsRef<[u8]>> {
    buffer: T,
}

mod field {
    use crate::wire::field::*;

    pub const FRAMECONTROL: Field = 0..2;
    pub const SEQUENCE_NUMBER: usize = 2;
    pub const ADDRESSING: Rest = 3..;
}

macro_rules! fc_bit_field {
    ($field:ident, $bit:literal) => {
        #[inline]
        pub fn $field(&self) -> bool {
            let data = self.buffer.as_ref();
            let raw = LittleEndian::read_u16(&data[field::FRAMECONTROL]);

            ((raw >> $bit) & 0b1) == 0b1
        }
    };
}

macro_rules! set_fc_bit_field {
    ($field:ident, $bit:literal) => {
        #[inline]
        pub fn $field(&mut self, val: bool) {
            let data = &mut self.buffer.as_mut()[field::FRAMECONTROL];
            let mut raw = LittleEndian::read_u16(data);
            raw |= ((val as u16) << $bit);

            data.copy_from_slice(&raw.to_le_bytes());
        }
    };
}

impl<T: AsRef<[u8]>> Frame<T> {
    /// Input a raw octet buffer with Ethernet frame structure.
    pub fn new_unchecked(buffer: T) -> Frame<T> {
        Frame { buffer }
    }

    /// Shorthand for a combination of [new_unchecked] and [check_len].
    ///
    /// [new_unchecked]: #method.new_unchecked
    /// [check_len]: #method.check_len
    pub fn new_checked(buffer: T) -> Result<Frame<T>> {
        let packet = Self::new_unchecked(buffer);
        packet.check_len()?;

        if matches!(packet.dst_addressing_mode(), AddressingMode::Unknown(_)) {
            return Err(Error::Malformed);
        }

        if matches!(packet.src_addressing_mode(), AddressingMode::Unknown(_)) {
            return Err(Error::Malformed);
        }

        Ok(packet)
    }

    /// Ensure that no accessor method will panic if called.
    /// Returns `Err(Error::Truncated)` if the buffer is too short.
    pub fn check_len(&self) -> Result<()> {
        if self.buffer.as_ref().is_empty() {
            Err(Error::Truncated)
        } else {
            Ok(())
        }
    }

    /// Consumes the frame, returning the underlying buffer.
    pub fn into_inner(self) -> T {
        self.buffer
    }

    /// Return the FrameType field.
    #[inline]
    pub fn frame_type(&self) -> FrameType {
        let data = self.buffer.as_ref();
        let raw = LittleEndian::read_u16(&data[field::FRAMECONTROL]);
        let ft = (raw & 0b11) as u8;
        FrameType::from(ft)
    }

    fc_bit_field!(security_enabled, 3);
    fc_bit_field!(frame_pending, 4);
    fc_bit_field!(ack_request, 5);
    fc_bit_field!(pan_id_compression, 6);

    fc_bit_field!(sequence_number_suppression, 8);
    fc_bit_field!(ie_present, 9);

    /// Return the destination addressing mode.
    #[inline]
    pub fn dst_addressing_mode(&self) -> AddressingMode {
        let data = self.buffer.as_ref();
        let raw = LittleEndian::read_u16(&data[field::FRAMECONTROL]);
        let am = ((raw >> 10) & 0b11) as u8;
        AddressingMode::from(am)
    }

    /// Return the frame version.
    #[inline]
    pub fn frame_version(&self) -> FrameVersion {
        let data = self.buffer.as_ref();
        let raw = LittleEndian::read_u16(&data[field::FRAMECONTROL]);
        let fv = ((raw >> 12) & 0b11) as u8;
        FrameVersion::from(fv)
    }

    /// Return the source addressing mode.
    #[inline]
    pub fn src_addressing_mode(&self) -> AddressingMode {
        let data = self.buffer.as_ref();
        let raw = LittleEndian::read_u16(&data[field::FRAMECONTROL]);
        let am = ((raw >> 14) & 0b11) as u8;
        AddressingMode::from(am)
    }

    /// Return the sequence number of the frame.
    #[inline]
    pub fn sequence_number(&self) -> Option<u8> {
        match self.frame_type() {
            FrameType::Beacon
            | FrameType::Data
            | FrameType::Acknowledgement
            | FrameType::MacCommand
            | FrameType::Multipurpose => {
                let data = self.buffer.as_ref();
                let raw = data[field::SEQUENCE_NUMBER];
                Some(raw)
            }
            FrameType::Extended | FrameType::FragmentOrFrak | FrameType::Unknown(_) => None,
        }
    }

    /// Return the addressing fields.
    #[inline]
    fn addressing_fields(&self) -> Option<&[u8]> {
        match self.frame_type() {
            FrameType::Beacon
            | FrameType::Data
            | FrameType::MacCommand
            | FrameType::Multipurpose => (),
            FrameType::Acknowledgement if self.frame_version() == FrameVersion::Ieee802154 => (),
            FrameType::Acknowledgement
            | FrameType::Extended
            | FrameType::FragmentOrFrak
            | FrameType::Unknown(_) => return None,
        }

        let mut offset = 2;

        // Calculate the size of the addressing field.
        offset += self.dst_addressing_mode().size();
        offset += self.src_addressing_mode().size();

        if !self.pan_id_compression() {
            offset += 2;
        }

        Some(&self.buffer.as_ref()[field::ADDRESSING][..offset])
    }

    /// Return the destination PAN field.
    #[inline]
    pub fn dst_pan_id(&self) -> Option<Pan> {
        let addressing_fields = self.addressing_fields()?;
        match self.dst_addressing_mode() {
            AddressingMode::Absent => None,
            AddressingMode::Short | AddressingMode::Extended => {
                Some(Pan(LittleEndian::read_u16(&addressing_fields[0..2])))
            }
            AddressingMode::Unknown(_) => None,
        }
    }

    /// Return the destination address field.
    #[inline]
    pub fn dst_addr(&self) -> Option<Address> {
        let addressing_fields = self.addressing_fields()?;
        match self.dst_addressing_mode() {
            AddressingMode::Absent => Some(Address::Absent),
            AddressingMode::Short => {
                let mut raw = [0u8; 2];
                raw.clone_from_slice(&addressing_fields[2..4]);
                raw.reverse();
                Some(Address::short_from_bytes(raw))
            }
            AddressingMode::Extended => {
                let mut raw = [0u8; 8];
                raw.clone_from_slice(&addressing_fields[2..10]);
                raw.reverse();
                Some(Address::extended_from_bytes(raw))
            }
            AddressingMode::Unknown(_) => None,
        }
    }

    /// Return the destination PAN field.
    #[inline]
    pub fn src_pan_id(&self) -> Option<Pan> {
        if self.pan_id_compression() {
            return None;
        }

        let addressing_fields = self.addressing_fields()?;
        let offset = self.dst_addressing_mode().size() + 2;

        match self.src_addressing_mode() {
            AddressingMode::Absent => None,
            AddressingMode::Short | AddressingMode::Extended => Some(Pan(LittleEndian::read_u16(
                &addressing_fields[offset..offset + 2],
            ))),
            AddressingMode::Unknown(_) => None,
        }
    }

    /// Return the source address field.
    #[inline]
    pub fn src_addr(&self) -> Option<Address> {
        let addressing_fields = self.addressing_fields()?;
        let mut offset = match self.dst_addressing_mode() {
            AddressingMode::Absent => 0,
            AddressingMode::Short => 2,
            AddressingMode::Extended => 8,
            _ => return None, // TODO(thvdveld): what do we do here?
        } + 2;

        if !self.pan_id_compression() {
            offset += 2;
        }

        match self.src_addressing_mode() {
            AddressingMode::Absent => Some(Address::Absent),
            AddressingMode::Short => {
                let mut raw = [0u8; 2];
                raw.clone_from_slice(&addressing_fields[offset..offset + 2]);
                raw.reverse();
                Some(Address::short_from_bytes(raw))
            }
            AddressingMode::Extended => {
                let mut raw = [0u8; 8];
                raw.clone_from_slice(&addressing_fields[offset..offset + 8]);
                raw.reverse();
                Some(Address::extended_from_bytes(raw))
            }
            AddressingMode::Unknown(_) => None,
        }
    }

    /// Return the Auxilliary Security Header Field
    #[inline]
    pub fn aux_security_header(&self) -> Option<&[u8]> {
        match self.frame_type() {
            FrameType::Beacon
            | FrameType::Data
            | FrameType::MacCommand
            | FrameType::Multipurpose => (),
            FrameType::Acknowledgement if self.frame_version() == FrameVersion::Ieee802154 => (),
            FrameType::Acknowledgement
            | FrameType::Extended
            | FrameType::FragmentOrFrak
            | FrameType::Unknown(_) => return None,
        }

        if !self.security_enabled() {
            return None;
        }

        net_debug!("Auxilliary security header is currently not supported.");
        None
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Frame<&'a T> {
    /// Return a pointer to the payload.
    #[inline]
    pub fn payload(&self) -> Option<&'a [u8]> {
        match self.frame_type() {
            FrameType::Data => {
                let data = &self.buffer.as_ref()[field::ADDRESSING];
                let offset = self.addressing_fields().unwrap().len();

                Some(&data[offset..])
            }
            _ => None,
        }
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Frame<T> {
    /// Set the frame type.
    #[inline]
    pub fn set_frame_type(&mut self, frame_type: FrameType) {
        let data = &mut self.buffer.as_mut()[field::FRAMECONTROL];
        let mut raw = LittleEndian::read_u16(data);

        raw = (raw & !(0b111)) | (u8::from(frame_type) as u16 & 0b111);
        data.copy_from_slice(&raw.to_le_bytes());
    }

    set_fc_bit_field!(set_security_enabled, 3);
    set_fc_bit_field!(set_frame_pending, 4);
    set_fc_bit_field!(set_ack_request, 5);
    set_fc_bit_field!(set_pan_id_compression, 6);

    /// Set the frame version.
    #[inline]
    pub fn set_frame_version(&mut self, version: FrameVersion) {
        let data = &mut self.buffer.as_mut()[field::FRAMECONTROL];
        let mut raw = LittleEndian::read_u16(data);

        raw = (raw & !(0b11 << 12)) | ((u8::from(version) as u16 & 0b11) << 12);
        data.copy_from_slice(&raw.to_le_bytes());
    }

    /// Set the frame sequence number.
    #[inline]
    pub fn set_sequence_number(&mut self, value: u8) {
        let data = self.buffer.as_mut();
        data[field::SEQUENCE_NUMBER] = value;
    }

    /// Set the destination PAN ID.
    #[inline]
    pub fn set_dst_pan_id(&mut self, value: Pan) {
        // NOTE the destination addressing mode must be different than Absent.
        // This is the reason why we set it to Extended.
        self.set_dst_addressing_mode(AddressingMode::Extended);

        let data = self.buffer.as_mut();
        data[field::ADDRESSING][..2].copy_from_slice(&value.as_bytes());
    }

    /// Set the destination address.
    #[inline]
    pub fn set_dst_addr(&mut self, mut value: Address) {
        match value {
            Address::Absent => self.set_dst_addressing_mode(AddressingMode::Absent),
            Address::Short(ref mut value) => {
                value.reverse();
                self.set_dst_addressing_mode(AddressingMode::Short);
                let data = self.buffer.as_mut();
                data[field::ADDRESSING][2..2 + 2].copy_from_slice(value);
                value.reverse();
            }
            Address::Extended(ref mut value) => {
                value.reverse();
                self.set_dst_addressing_mode(AddressingMode::Extended);
                let data = &mut self.buffer.as_mut()[field::ADDRESSING];
                data[2..2 + 8].copy_from_slice(value);
                value.reverse();
            }
        }
    }

    /// Set the destination addressing mode.
    #[inline]
    fn set_dst_addressing_mode(&mut self, value: AddressingMode) {
        let data = &mut self.buffer.as_mut()[field::FRAMECONTROL];
        let mut raw = LittleEndian::read_u16(data);

        raw = (raw & !(0b11 << 10)) | ((u8::from(value) as u16 & 0b11) << 10);
        data.copy_from_slice(&raw.to_le_bytes());
    }

    /// Set the source PAN ID.
    #[inline]
    pub fn set_src_pan_id(&mut self, value: Pan) {
        let offset = match self.dst_addressing_mode() {
            AddressingMode::Absent => 0,
            AddressingMode::Short => 2,
            AddressingMode::Extended => 8,
            _ => unreachable!(),
        } + 2;

        let data = &mut self.buffer.as_mut()[field::ADDRESSING];
        data[offset..offset + 2].copy_from_slice(&value.as_bytes());
    }

    /// Set the source address.
    #[inline]
    pub fn set_src_addr(&mut self, mut value: Address) {
        let offset = match self.dst_addressing_mode() {
            AddressingMode::Absent => 0,
            AddressingMode::Short => 2,
            AddressingMode::Extended => 8,
            _ => unreachable!(),
        } + 2;

        let offset = offset + if self.pan_id_compression() { 0 } else { 2 };

        match value {
            Address::Absent => self.set_src_addressing_mode(AddressingMode::Absent),
            Address::Short(ref mut value) => {
                value.reverse();
                self.set_src_addressing_mode(AddressingMode::Short);
                let data = &mut self.buffer.as_mut()[field::ADDRESSING];
                data[offset..offset + 2].copy_from_slice(value);
                value.reverse();
            }
            Address::Extended(ref mut value) => {
                value.reverse();
                self.set_src_addressing_mode(AddressingMode::Extended);
                let data = &mut self.buffer.as_mut()[field::ADDRESSING];
                data[offset..offset + 8].copy_from_slice(value);
                value.reverse();
            }
        }
    }

    /// Set the source addressing mode.
    #[inline]
    fn set_src_addressing_mode(&mut self, value: AddressingMode) {
        let data = &mut self.buffer.as_mut()[field::FRAMECONTROL];
        let mut raw = LittleEndian::read_u16(data);

        raw = (raw & !(0b11 << 14)) | ((u8::from(value) as u16 & 0b11) << 14);
        data.copy_from_slice(&raw.to_le_bytes());
    }

    /// Return a mutable pointer to the payload.
    #[inline]
    pub fn payload_mut(&mut self) -> Option<&mut [u8]> {
        match self.frame_type() {
            FrameType::Data => {
                let mut start_offset = 3;
                start_offset += self.addressing_fields().unwrap().len();

                let data = self.buffer.as_mut();
                let end_offset = start_offset + data.len() - 2;
                Some(&mut data[start_offset..end_offset])
            }
            _ => None,
        }
    }
}

impl<T: AsRef<[u8]>> fmt::Display for Frame<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "IEEE802.15.4 frame type={} seq={:2x?} dst_pan={:x?} dest={:x?} src_pan={:?} src={:x?}",
            self.frame_type(),
            self.sequence_number(),
            self.dst_pan_id(),
            self.dst_addr(),
            self.src_pan_id(),
            self.src_addr(),
        )
    }
}

/// A high-level representation of an IEEE802.15.4 frame.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Repr {
    pub frame_type: FrameType,
    pub security_enabled: bool,
    pub frame_pending: bool,
    pub ack_request: bool,
    pub sequence_number: Option<u8>,
    pub pan_id_compression: bool,
    pub frame_version: FrameVersion,
    pub dst_pan_id: Option<Pan>,
    pub dst_addr: Option<Address>,
    pub src_pan_id: Option<Pan>,
    pub src_addr: Option<Address>,
}

impl Repr {
    /// Parse an IEEE 802.15.4 frame and return a high-level representation.
    pub fn parse<T: AsRef<[u8]> + ?Sized>(packet: &Frame<&T>) -> Result<Repr> {
        // Ensure the basic accessors will work.
        packet.check_len()?;

        Ok(Repr {
            frame_type: packet.frame_type(),
            security_enabled: packet.security_enabled(),
            frame_pending: packet.frame_pending(),
            ack_request: packet.ack_request(),
            sequence_number: packet.sequence_number(),
            pan_id_compression: packet.pan_id_compression(),
            frame_version: packet.frame_version(),
            dst_pan_id: packet.dst_pan_id(),
            dst_addr: packet.dst_addr(),
            src_pan_id: packet.src_pan_id(),
            src_addr: packet.src_addr(),
        })
    }

    /// Return the length of a buffer required to hold a packet with the payload of a given length.
    #[inline]
    pub fn buffer_len(&self) -> usize {
        3 + 2
            + match self.dst_addr {
                Some(Address::Absent) | None => 0,
                Some(Address::Short(_)) => 2,
                Some(Address::Extended(_)) => 8,
            }
            + if !self.pan_id_compression { 2 } else { 0 }
            + match self.src_addr {
                Some(Address::Absent) | None => 0,
                Some(Address::Short(_)) => 2,
                Some(Address::Extended(_)) => 8,
            }
    }

    /// Emit a high-level representation into an IEEE802.15.4 frame.
    pub fn emit<T: AsRef<[u8]> + AsMut<[u8]>>(&self, frame: &mut Frame<T>) {
        frame.set_frame_type(self.frame_type);
        frame.set_security_enabled(self.security_enabled);
        frame.set_frame_pending(self.frame_pending);
        frame.set_ack_request(self.ack_request);
        frame.set_pan_id_compression(self.pan_id_compression);
        frame.set_frame_version(self.frame_version);

        if let Some(sequence_number) = self.sequence_number {
            frame.set_sequence_number(sequence_number);
        }

        if let Some(dst_pan_id) = self.dst_pan_id {
            frame.set_dst_pan_id(dst_pan_id);
        }
        if let Some(dst_addr) = self.dst_addr {
            frame.set_dst_addr(dst_addr);
        }

        if !self.pan_id_compression && self.src_pan_id.is_some() {
            frame.set_src_pan_id(self.src_pan_id.unwrap());
        }

        if let Some(src_addr) = self.src_addr {
            frame.set_src_addr(src_addr);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Result;

    #[test]
    fn test_broadcast() {
        assert!(Address::BROADCAST.is_broadcast());
        assert!(!Address::BROADCAST.is_unicast());
    }

    #[test]
    fn prepare_frame() {
        let mut buffer = [0u8; 128];

        let repr = Repr {
            frame_type: FrameType::Data,
            security_enabled: false,
            frame_pending: false,
            ack_request: true,
            pan_id_compression: true,
            frame_version: FrameVersion::Ieee802154,
            sequence_number: Some(1),
            dst_pan_id: Some(Pan(0xabcd)),
            dst_addr: Some(Address::BROADCAST),
            src_pan_id: None,
            src_addr: Some(Address::Extended([
                0xc7, 0xd9, 0xb5, 0x14, 0x00, 0x4b, 0x12, 0x00,
            ])),
        };

        let buffer_len = repr.buffer_len();

        let mut frame = Frame::new_unchecked(&mut buffer[..buffer_len]);
        repr.emit(&mut frame);

        println!("{:2x?}", frame);

        assert_eq!(frame.frame_type(), FrameType::Data);
        assert!(!frame.security_enabled());
        assert!(!frame.frame_pending());
        assert!(frame.ack_request());
        assert!(frame.pan_id_compression());
        assert_eq!(frame.frame_version(), FrameVersion::Ieee802154);
        assert_eq!(frame.sequence_number(), Some(1));
        assert_eq!(frame.dst_pan_id(), Some(Pan(0xabcd)));
        assert_eq!(frame.dst_addr(), Some(Address::BROADCAST));
        assert_eq!(frame.src_pan_id(), None);
        assert_eq!(
            frame.src_addr(),
            Some(Address::Extended([
                0xc7, 0xd9, 0xb5, 0x14, 0x00, 0x4b, 0x12, 0x00
            ]))
        );
    }

    macro_rules! vector_test {
        ($name:ident $bytes:expr ; $($test_method:ident -> $expected:expr,)*) => {
            #[test]
            #[allow(clippy::bool_assert_comparison)]
            fn $name() -> Result<()> {
                let frame = &$bytes;
                let frame = Frame::new_checked(frame)?;

                $(
                    assert_eq!(frame.$test_method(), $expected, stringify!($test_method));
                )*

                Ok(())
            }
        }
    }

    vector_test! {
        extended_addr
        [
            0b0000_0001, 0b1100_1100, // frame control
            0b0, // seq
            0xcd, 0xab, // pan id
            0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, // dst addr
            0x03, 0x04, // pan id
            0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, // src addr
        ];
        frame_type -> FrameType::Data,
        dst_addr -> Some(Address::Extended([0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00])),
        src_addr -> Some(Address::Extended([0x02, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00])),
        dst_pan_id -> Some(Pan(0xabcd)),
    }

    vector_test! {
        short_addr
        [
            0x01, 0x98,             // frame control
            0x00,                   // sequence number
            0x34, 0x12, 0x78, 0x56, // PAN identifier and address of destination
            0x34, 0x12, 0xbc, 0x9a, // PAN identifier and address of source
        ];
        frame_type -> FrameType::Data,
        security_enabled -> false,
        frame_pending -> false,
        ack_request -> false,
        pan_id_compression -> false,
        dst_addressing_mode -> AddressingMode::Short,
        frame_version -> FrameVersion::Ieee802154_2006,
        src_addressing_mode -> AddressingMode::Short,
        dst_pan_id -> Some(Pan(0x1234)),
        dst_addr -> Some(Address::Short([0x56, 0x78])),
        src_pan_id -> Some(Pan(0x1234)),
        src_addr -> Some(Address::Short([0x9a, 0xbc])),
    }

    vector_test! {
        zolertia_remote
        [
            0x41, 0xd8, // frame control
            0x01, // sequence number
            0xcd, 0xab, // Destination PAN id
            0xff, 0xff, // Short destination address
            0xc7, 0xd9, 0xb5, 0x14, 0x00, 0x4b, 0x12, 0x00, // Extended source address
            0x2b, 0x00, 0x00, 0x00, // payload
        ];
        frame_type -> FrameType::Data,
        security_enabled -> false,
        frame_pending -> false,
        ack_request -> false,
        pan_id_compression -> true,
        dst_addressing_mode -> AddressingMode::Short,
        frame_version -> FrameVersion::Ieee802154_2006,
        src_addressing_mode -> AddressingMode::Extended,
        //payload -> Some(&[0x2b, 0x00, 0x00, 0x00]),
    }
}
