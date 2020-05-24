use arsdk_rs::{
    command::Feature,
    frame::{BufferID, Frame as InnerFrame, Type},
};


pub struct Frame<S: FrameState> {
    state: S,
}

struct SetFrameType {}

struct SetBufferId {
    frame_type: Type,
}

struct SetFeature {
    frame_type: Type,
    buffer_id: BufferID,
}

struct ReceiveFeature {
    frame_type: Type,
    receive_buffer_id: ReceiveBufferId,
}

struct SendFeature {
    frame_type: Type,
    send_buffer_id: SendBufferId,
}

pub trait FrameState {}

impl FrameState for SetFrameType {}
impl FrameState for SetBufferId {}
impl FrameState for SetFeature {}
impl FrameState for SendFeature {}
impl FrameState for ReceiveFeature {}
impl FrameState for InnerFrame {}

impl Frame<SetFrameType> {
    pub fn new() -> Self {
        Self {
            state: SetFrameType {},
        }
    }

    pub fn frame_type(self, frame_type: Type) -> Frame<SetBufferId> {

        Frame {
            state: SetBufferId { frame_type },
        }
    }
}


impl Frame<SetBufferId> {
    pub fn send(self, send: SendBufferId) -> Frame<SendFeature> {

        Frame {
            state: SendFeature {
                frame_type: self.state.frame_type,
                send_buffer_id: send,
            },
        }
    }

    pub fn receive(self, receive: ReceiveBufferId) -> Frame<ReceiveFeature> {
        Frame {
            state: ReceiveFeature {
                frame_type: self.state.frame_type,
                receive_buffer_id: receive,
            },
        }
    }

    pub fn buffer_id(self, buffer_id: BufferID) -> Frame<SetFeature> {
        Frame {
            state: SetFeature {
                frame_type: self.state.frame_type,
                buffer_id,
            },
        }
    }
}




pub enum SendBufferId {
    Pong = 1,
    NoAcknowledge = 10,
    Acknowledge = 11,
    Emergency = 12,
    VideoAcknowledge = 13,
}

pub enum ReceiveBufferId {
    Ping = 0,
    Video = 125,
    Event = 126,
    Navigation = 127,
    Acknowledge = 139,
}

impl Into<BufferID> for SendBufferId {
    fn into(self) -> BufferID {
        match self {
            SendBufferId::Pong => BufferID::PONG,
            SendBufferId::NoAcknowledge => BufferID::CDNonAck,
            SendBufferId::Acknowledge => BufferID::CDAck,
            SendBufferId::Emergency => BufferID::CDEmergency,
            SendBufferId::VideoAcknowledge => BufferID::CDVideoAck,
        }
    }
}

impl Into<BufferID> for ReceiveBufferId {
    fn into(self) -> BufferID {
        match self {
            ReceiveBufferId::Ping => BufferID::PING,
            ReceiveBufferId::Video => BufferID::DCVideo,
            ReceiveBufferId::Event => BufferID::DCEvent,
            ReceiveBufferId::Navigation => BufferID::DCNavdata,
            ReceiveBufferId::Acknowledge => BufferID::ACKFromSendWithAck,
        }
    }
}

impl Frame<SetFeature> {
    pub fn feature(self, sequence: u8, feature: Feature) -> InnerFrame {
        InnerFrame {
            frame_type: self.state.frame_type,
            buffer_id: self.state.buffer_id,
            sequence_id: sequence,
            feature: Some(feature),
        }
    }
}

impl Frame<SendFeature> {
    pub fn feature(self, sequence: u8, feature: Feature) -> InnerFrame {
        InnerFrame {
            frame_type: self.state.frame_type,
            buffer_id: self.state.send_buffer_id.into(),
            sequence_id: sequence,
            feature: Some(feature),
        }
    }
}

impl Frame<ReceiveFeature> {
    pub fn feature(self, sequence: u8, feature: Feature) -> InnerFrame {
        InnerFrame {
            frame_type: self.state.frame_type,
            buffer_id: self.state.receive_buffer_id.into(),
            sequence_id: sequence,
            feature: Some(feature),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use arsdk_rs::jumping_sumo::{Class, Anim};
    use scroll::{Pread, Pwrite, LE};

    #[test]
    fn build_a_frame() {
        let feature = Feature::JumpingSumo(Class::Animations(Anim::Jump));

        let frame = Frame::new()
            .frame_type(Type::DataWithAck)
            .send(SendBufferId::Acknowledge)
            .feature(1, feature);

        let message: [u8; 15] = [
            0x4, 0xb, 0x1, 0xf, 0x0, 0x0, 0x0, 0x3, 0x2, 0x3, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];

        assert_frames_match(&message, frame);
    }

    // Copy-pasted from `arsdk-rs`
    fn assert_frames_match(expected: &[u8], frame: InnerFrame) {
        // Check the value at the Frame length bytes 3 to 7
        let buf_len: u32 = (&expected[3..7])
            .pread_with(0, LE)
            .expect("should read a u32");

        assert_eq!(buf_len as usize, expected.len());

        // Deserialize a Frame
        assert_eq!(
            frame,
            expected
                .pread_with::<InnerFrame>(0, LE)
                .expect("Should deserialize"),
        );
        let mut actual = [0_u8; 256];
        assert!(
            actual.len() > buf_len as usize,
            "Whoopsy... our serialization buffer is not that big!"
        );

        let mut offset = 0;
        let actual_written = actual
            .gwrite_with(frame, &mut offset, LE)
            .expect("Should serialize");

        assert_eq!(expected, &actual[..offset]);
        assert_eq!(buf_len as usize, actual_written);
    }
}
