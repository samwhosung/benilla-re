//! The world connection's client object (`NetClient.cpp`; `systems/net/session.md`,
//! `systems/net/dispatch.md`, `systems/net/wire.md`): the keepalive, the ping ring and its
//! statistics, sending, and the dispatch of a received packet.

/// `CMSG_PING`.
pub const CMSG_PING: u32 = 0x1dc;

/// The `NetClient` fields the keepalive and the statistics use.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PingState {
    /// `+0x70`: the session state; 6 is connected.
    pub state: u32,
    /// `+0x1a64`: the clock at the last ping or connect.
    pub last_ping: u32,
    /// `+0x1a68`: the last ping's sequence number.
    pub seq: u32,
    /// `+0x1a6c`: the latency samples.
    pub ring: [u32; 16],
    /// `+0x1aac` and `+0x1ab0`: the ring's read and write indices.
    pub read_idx: u32,
    pub write_idx: u32,
    /// `+0x1ab4` and `+0x1ab8`: the bytes sent and received.
    pub sent: u32,
    pub received: u32,
    /// `+0x1abc`: the clock when the connection came up.
    pub connected_at: u32,
}

impl PingState {
    /// `NetClient::HandleIdle` (`0x537ff0`): whether it pings, when `now - last_ping - 30000`
    /// is not negative as a signed dword.
    pub fn idle_due(&self, now: u32) -> bool {
        (now.wrapping_sub(self.last_ping).wrapping_sub(30000) as i32) >= 0
    }

    /// `NetClient::Ping` (`0x537e10`): stamps `last_ping`, increments `seq`, and builds the packet
    /// it passes to `NetClient::Send`: the opcode, the sequence number and, in state 6, the
    /// sample before the write index, or 0 when the index is 0.
    pub fn ping(&mut self, now: u32) -> Vec<u8> {
        self.last_ping = now;
        self.seq = self.seq.wrapping_add(1);
        let mut p = CMSG_PING.to_le_bytes().to_vec();
        p.extend(self.seq.to_le_bytes());
        if self.state == 6 {
            let w = self.write_idx;
            let v = if w == 0 {
                0
            } else {
                self.ring[(w - 1) as usize]
            };
            p.extend(v.to_le_bytes());
        }
        p
    }

    /// `NetClient::HandlePong` (`0x537d60`) with the pong's sequence number: a number other than
    /// `seq` is logged and ignored (`false`); otherwise `now - last_ping` goes into the ring at
    /// the write index, which advances modulo 16, pushing the read index on when they meet.
    pub fn pong(&mut self, value: u32, now: u32) -> bool {
        if value != self.seq {
            return false;
        }
        self.ring[self.write_idx as usize] = now.wrapping_sub(self.last_ping);
        self.write_idx += 1;
        if self.write_idx >= 16 {
            self.write_idx = 0;
        }
        if self.write_idx == self.read_idx {
            self.read_idx += 1;
            if self.read_idx >= 16 {
                self.read_idx = 0;
            }
        }
        true
    }

    /// `NetClient::GetNetStats` (`0x537f20`): the received and sent rates and the mean latency.
    /// The seconds are `(now - connected_at)` as an unsigned dword times the `f32` 0.001
    /// (`0x3a83126f`), each byte count times 1/1024 is divided by them in double precision and
    /// stored as `f32`. The mean walks the ring from the read index to the write index, wrapping
    /// at 16 but stopping there when the write index is 0, and divides the wrapping sum by the
    /// count, or is 0.
    pub fn net_stats(&self, now: u32) -> (f32, f32, u32) {
        let secs =
            f64::from(now.wrapping_sub(self.connected_at)) * f64::from(f32::from_bits(0x3a83_126f));
        let kib = f64::from(f32::from_bits(0x3a80_0000));
        let recv = ((f64::from(self.received) * kib) / secs) as f32;
        let sent = ((f64::from(self.sent) * kib) / secs) as f32;
        let (mut i, w) = (self.read_idx, self.write_idx);
        let (mut sum, mut n) = (0u32, 0u32);
        if i != w {
            loop {
                if i >= 16 {
                    i = 0;
                    if w == 0 {
                        break;
                    }
                }
                sum = sum.wrapping_add(self.ring[i as usize]);
                n += 1;
                i += 1;
                if i == w {
                    break;
                }
            }
        }
        (recv, sent, sum.checked_div(n).unwrap_or(0))
    }
}

/// What `NetClient::Send` (`0x5379a0`) does with a packet of `pending` unread bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SendEffect {
    /// The packet goes to `WowConnection::Send` (`0x5b5630`) and `pending` is added to the
    /// global sent count `0xc0d45c` and to `+0x1ab4`.
    pub sent: bool,
    /// The cipher is armed after the send: key `+0x48`, key length 40, send span 4, receive
    /// span 2.
    pub arm: bool,
}

/// `NetClient::Send`: only in state 6 and with bytes left to read; arms the cipher when the
/// connection's `+0x120` is clear.
pub fn net_send(state: u32, pending: u32, armed: bool) -> SendEffect {
    let sent = state == 6 && pending != 0;
    SendEffect {
        sent,
        arm: sent && !armed,
    }
}

/// The 16-bit opcode `NetClient::DispatchMessage` (`0x537aa0`) reads from a packet of at least
/// two bytes, which indexes the handler table at `+0x74` and the context table at `+0xd64`.
pub fn dispatch_opcode(packet: &[u8]) -> u16 {
    u16::from_le_bytes([packet[0], packet[1]])
}

/// What `NetClient::Initialize` (`0x537600`) does, given the count of initialized clients
/// (`0xc0d4b4`) and, for the first client, what the network platform's start (`0x5b7080`)
/// returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InitializeEffect {
    /// Whether the platform start runs: only for the first client.
    pub starts_platform: bool,
    /// The result: 0 when the platform failed to start, else 1 with the count incremented, the
    /// event queue (40 bytes, line `0x84`) and the connection (`0x130` bytes, line `0x88`)
    /// allocated and built, both tables zeroed, the state 2 and the clock stored in `0xc0d474`.
    pub result: u32,
    pub count: u32,
}

pub fn net_initialize(count: u32, platform_ok: bool) -> InitializeEffect {
    let starts_platform = count == 0;
    if starts_platform && !platform_ok {
        return InitializeEffect {
            starts_platform,
            result: 0,
            count,
        };
    }
    InitializeEffect {
        starts_platform,
        result: 1,
        count: count.wrapping_add(1),
    }
}

/// What `NetClient::Deinitialize` (`0x5376f0`) does, given the state and the count of
/// initialized clients.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeinitializeEffect {
    /// In state 0 it does nothing. Otherwise it disconnects (`0x537910`), closes and releases the
    /// connection, zeroes both tables, destroys and frees the queue, decrements the count, stops
    /// the platform when the count reaches 0 (`0x429d50(1)`, `0x5b7140`) and sets state 0.
    pub runs: bool,
    pub count: u32,
    pub stops_platform: bool,
    /// The tail call to `0x5377a0` when the count is 0 at the end.
    pub tail: bool,
}

pub fn net_deinitialize(state: u32, count: u32) -> DeinitializeEffect {
    if state == 0 {
        return DeinitializeEffect {
            runs: false,
            count,
            stops_platform: false,
            tail: false,
        };
    }
    let count = count.wrapping_sub(1);
    DeinitializeEffect {
        runs: true,
        count,
        stops_platform: count == 0,
        tail: count == 0,
    }
}

/// The words `NETEVENTQUEUE::AddEvent` (`0x538790`, `(type, unused, client, data, length)`)
/// writes for a node it allocates at `node`, the queue's last link at `+0x20` being `tail`,
/// whose `+4` holds `tail_next`: the node links after `tail` and becomes the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueuedNode {
    /// The node's `+0` and `+4`: `tail` and `tail_next`.
    pub links: (u32, u32),
    /// `tail + 4` and the queue's `+0x20` both become `node`.
    pub kind: u32,
    /// `+0xc`: the clock.
    pub time: u32,
    /// `+0x14`: the length; `+0x10` holds a copy of the data, or 0 for a length of 0.
    pub len: u32,
}

pub fn queue_event(tail: u32, tail_next: u32, kind: u32, len: u32, now: u32) -> QueuedNode {
    QueuedNode {
        links: (tail, tail_next),
        kind,
        time: now,
        len,
    }
}
