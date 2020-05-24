use arsdk_rs::{
    command::Feature,
    frame::{BufferID, Type, Frame as InnerFrame},
};
use std::fmt::Debug;

pub type RawFrame = Vec<Box<dyn FramePart>>;

pub trait FramePart: Debug {}
impl FramePart for InnerFrame {}
impl FramePart for Type {}
impl FramePart for BufferID {}
impl FramePart for Feature {}

pub struct Frame<S: FrameState> {
    state: Box<RawFrame>,
    extra: S,
}

struct SetFrameType {}

struct SetBufferId {
    frame_type: Type,
}

struct SetFeature {
    frame_type: Type,
    buffer_id: BufferID,
}

pub trait FrameState {}

impl FrameState for SetFrameType {}
impl FrameState for SetBufferId {}
impl FrameState for SetFeature {}
impl FrameState for InnerFrame {}

impl Frame<SetFrameType> {
    pub fn new() -> Self {
        Self {
            state: Box::new(RawFrame::new()),
            extra: SetFrameType {}
        }
    }

    pub fn frame_type(mut self, frame_type: Type) -> Frame<SetBufferId> {
        self.state.push(Box::new(frame_type));

        Frame {
            state: self.state,
            extra: SetBufferId { frame_type },
        }
    }
}

impl Frame<SetBufferId> {
    pub fn buffer_id(mut self, buffer_id: BufferID) -> Frame<SetFeature> {
        self.state.push(Box::new(buffer_id));

        Frame {
            state: self.state,
            extra: SetFeature {
                frame_type: self.extra.frame_type,
                buffer_id,
            },
        }
    }
}

impl Frame<SetFeature> {
    pub fn feature(mut self, sequence: u8, feature: Feature) -> Frame<InnerFrame> {
        self.state.push(Box::new(feature.clone()));

        let inner = InnerFrame {
            frame_type: self.extra.frame_type,
            buffer_id: self.extra.buffer_id,
            sequence_id: sequence,
            feature: Some(feature),
        };

        Frame {
            state: self.state,
            extra: inner,
        }
    }
}

impl Frame<InnerFrame> {
    pub fn raw_frame(&self) -> &RawFrame {
        &self.state
    }

    pub fn as_frame(self) -> InnerFrame {
        self.extra
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use arsdk_rs::ardrone3::{ArDrone3, Piloting};
    use scroll::{Pread, LE, Pwrite};

    #[test]
    fn build_a_frame() {
        let feature = Feature::ArDrone3(Some(ArDrone3::Piloting(Piloting::TakeOff)));

        let frame = Frame::new()
        .frame_type(Type::DataWithAck).buffer_id(BufferID::CDAck).feature(1, feature);

        let message: [u8; 15] = [
            0x4, 0xb, 0x1, 0xf, 0x0, 0x0, 0x0, 0x3, 0x2, 0x3, 0x0, 0x0, 0x0, 0x0, 0x0,
        ];


        dbg!(frame.raw_frame());
        assert_frames_match(&message, frame.as_frame());
    }

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
