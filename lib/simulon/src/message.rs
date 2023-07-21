use std::cmp::Reverse;

use crate::{api::RemoteAddr, state::ResourceId};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Message {
    // Time must be the first field so the derived impl of ord works as expected.
    // Also we reverse it since we use a max heap but we want the messages with smaller
    // time to come first.
    pub time: Reverse<u128>,
    pub sender: RemoteAddr,
    pub receiver: RemoteAddr,
    pub detail: MessageDetail,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MessageDetail {
    Connect {
        port: u16,
        rid: ResourceId,
    },
    ConnectionAccepted {
        sender_rid: ResourceId,
        receiver_rid: ResourceId,
    },
    ConnectionRefused {
        receiver_rid: ResourceId,
    },
    ConnectionClosed {
        receiver_rid: ResourceId,
    },
    Data {
        receiver_rid: ResourceId,
        data: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::BinaryHeap;

    use super::*;
    use crate::state::ResourceId;

    #[test]
    fn test_ordering() {
        let m1 = Message {
            time: Reverse(0),
            sender: RemoteAddr(1),
            receiver: RemoteAddr(2),
            detail: super::MessageDetail::ConnectionClosed {
                receiver_rid: ResourceId(18),
            },
        };

        let m2 = Message {
            time: Reverse(17),
            sender: RemoteAddr(1),
            receiver: RemoteAddr(2),
            detail: super::MessageDetail::ConnectionClosed {
                receiver_rid: ResourceId(18),
            },
        };

        let m3 = Message {
            time: Reverse(3),
            sender: RemoteAddr(0),
            receiver: RemoteAddr(0),
            detail: super::MessageDetail::ConnectionClosed {
                receiver_rid: ResourceId(0),
            },
        };

        let m4 = Message {
            time: Reverse(5),
            sender: RemoteAddr(0),
            receiver: RemoteAddr(0),
            detail: super::MessageDetail::ConnectionClosed {
                receiver_rid: ResourceId(0),
            },
        };

        let m5 = Message {
            time: Reverse(2),
            sender: RemoteAddr(18),
            receiver: RemoteAddr(0),
            detail: super::MessageDetail::ConnectionClosed {
                receiver_rid: ResourceId(0),
            },
        };

        let mut set: BinaryHeap<Message> = BinaryHeap::new();

        set.push(m1);
        set.push(m2);
        set.push(m3);
        set.push(m4);
        set.push(m5);

        assert_eq!(set.pop().unwrap().time.0, 0);
        assert_eq!(set.pop().unwrap().time.0, 2);
        assert_eq!(set.pop().unwrap().time.0, 3);
        assert_eq!(set.pop().unwrap().time.0, 5);
        assert_eq!(set.pop().unwrap().time.0, 17);
    }
}
