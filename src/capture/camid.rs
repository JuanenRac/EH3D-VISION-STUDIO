//! The identifier of a camera that is the same on every run: `vendor:product:serial@port`, for example `046d:0825:A1B2C3@1-2.3`. The configuration names a camera by this key and not by the
//! number that the system gives it today, which changes when another camera is plugged in. A camera that has a serial number is the same camera on any port (the port that is found is said, so
//! that a camera that was moved is noticed); two cameras of one model that have no serial number are told apart by their port, and when the port has changed too the answer is "more than one",
//! never a guess.

use super::{CameraId, CameraInfo, CapError, Code};

impl CameraId {
    /// The key: `vvvv:pppp:serial@port` with the numbers in hexadecimal.
    pub fn key(&self) -> String {
        format!(
            "{:04x}:{:04x}:{}@{}",
            self.vendor, self.product, self.serial, self.port
        )
    }

    /// The identifier written by `key`. A serial or a port may be empty; none of the parts may contain `@`, `:` or spaces.
    pub fn parse(text: &str) -> Result<CameraId, CapError> {
        let bad = |why: &str| {
            CapError::with_key(
                Code::CameraNotFound,
                "capture_camera",
                format!("{text:?}: {why}"),
            )
        };
        let (head, port) = text
            .split_once('@')
            .ok_or_else(|| bad("it must be vendor:product:serial@port"))?;
        let mut parts = head.splitn(3, ':');
        let (vendor, product, serial) = (parts.next(), parts.next(), parts.next());
        let (Some(vendor), Some(product), Some(serial)) = (vendor, product, serial) else {
            return Err(bad("it must be vendor:product:serial@port"));
        };
        let hex = |s: &str| {
            if s.len() == 4 {
                u16::from_str_radix(s, 16).ok()
            } else {
                None
            }
        };
        let (Some(vendor), Some(product)) = (hex(vendor), hex(product)) else {
            return Err(bad(
                "the vendor and the product are four hexadecimal digits",
            ));
        };
        let clean = |s: &str| {
            s.len() <= 64
                && s.chars()
                    .all(|c| c.is_ascii_graphic() && !matches!(c, '@' | ':'))
        };
        if !clean(serial) || !clean(port) {
            return Err(bad(
                "the serial and the port are text without spaces, @ or :",
            ));
        }
        Ok(CameraId {
            vendor,
            product,
            serial: serial.to_string(),
            port: port.to_string(),
        })
    }

    /// Whether `other` is the same camera: the same model and the same serial number, or no serial number on either and the same port.
    pub fn same_device(&self, other: &CameraId) -> bool {
        if self.vendor != other.vendor || self.product != other.product {
            return false;
        }
        if self.serial.is_empty() && other.serial.is_empty() {
            return self.port == other.port;
        }
        self.serial == other.serial
    }
}

/// What was found about a camera that the configuration names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Where it was.
    Same,
    /// The same camera on another port.
    MovedPort { from: String, to: String },
}

/// The camera that `wanted` names among the listed ones, with whether it is where it was. No camera is `camera_not_found`; two that fit are `camera_not_found` too, with the number, because
/// choosing one would send the settings of one camera to another.
pub fn locate(wanted: &CameraId, listed: &[CameraInfo]) -> Result<(usize, Change), CapError> {
    let hits: Vec<usize> = listed
        .iter()
        .enumerate()
        .filter(|(_, c)| wanted.same_device(&c.id))
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [] => Err(CapError::with_key(
            Code::CameraNotFound,
            "capture_camera",
            format!("{} is not among the {} listed", wanted.key(), listed.len()),
        )),
        [i] => {
            let found = &listed[*i].id;
            let change = if found.port == wanted.port {
                Change::Same
            } else {
                Change::MovedPort {
                    from: wanted.port.clone(),
                    to: found.port.clone(),
                }
            };
            Ok((*i, change))
        }
        many => Err(CapError::with_key(
            Code::CameraNotFound,
            "capture_camera",
            format!(
                "{} fit {}: {} cameras (give the serial number or plug them in as they were)",
                wanted.key(),
                many.len(),
                many.len()
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(v: u16, p: u16, serial: &str, port: &str) -> CameraId {
        CameraId {
            vendor: v,
            product: p,
            serial: serial.into(),
            port: port.into(),
        }
    }

    fn info(i: CameraId) -> CameraInfo {
        CameraInfo {
            name: "x".into(),
            id: i,
            formats: Vec::new(),
        }
    }

    #[test]
    fn a_key_is_read_back_into_the_same_identifier() {
        for i in [
            id(0x046d, 0x0825, "A1B2", "1-2.3"),
            id(0x1234, 0x00ff, "", ""),
            id(0, 0, "S", "csi0"),
        ] {
            assert_eq!(CameraId::parse(&i.key()).unwrap(), i);
        }
        assert_eq!(id(0x46d, 0x825, "A", "1-2").key(), "046d:0825:A@1-2");
    }

    #[test]
    fn a_key_with_a_mistake_is_refused_and_names_the_key_of_the_configuration() {
        for bad in [
            "",
            "046d:0825:A",
            "046d@1-2",
            "46d:0825:A@1",
            "zzzz:0825:A@1",
            "046d:0825:A B@1",
            "046d:0825:A@1:2",
            "046d:0825:A@1 2",
        ] {
            let e = CameraId::parse(bad).unwrap_err();
            assert_eq!(e.code, Code::CameraNotFound, "{bad}");
            assert_eq!(e.key, Some("capture_camera"));
        }
        let long = format!("046d:0825:{}@1", "x".repeat(65));
        assert!(CameraId::parse(&long).is_err());
    }

    #[test]
    fn a_camera_with_a_serial_is_found_on_any_port_and_the_move_is_said() {
        let wanted = id(1, 2, "SER", "1-1");
        let listed = vec![
            info(id(1, 2, "OTHER", "1-1")),
            info(id(1, 2, "SER", "2-4.1")),
        ];
        let (at, change) = locate(&wanted, &listed).unwrap();
        assert_eq!(at, 1);
        assert_eq!(
            change,
            Change::MovedPort {
                from: "1-1".into(),
                to: "2-4.1".into()
            }
        );
        let (_, same) = locate(&wanted, &[info(id(1, 2, "SER", "1-1"))]).unwrap();
        assert_eq!(same, Change::Same);
    }

    #[test]
    fn two_cameras_without_a_serial_are_told_apart_by_the_port_and_never_guessed() {
        let a = id(1, 2, "", "1-1");
        let b = id(1, 2, "", "1-2");
        let listed = vec![info(a.clone()), info(b.clone())];
        assert_eq!(locate(&b, &listed).unwrap().0, 1);
        let moved = id(1, 2, "", "1-9");
        let e = locate(&moved, &listed).unwrap_err();
        assert_eq!(e.code, Code::CameraNotFound);
        let twins = vec![info(a.clone()), info(a.clone())];
        let e = locate(&a, &twins).unwrap_err();
        assert!(e.detail.contains("2 cameras"), "{e}");
    }

    #[test]
    fn a_camera_of_another_model_is_never_taken_for_the_wanted_one() {
        let wanted = id(1, 2, "SER", "1-1");
        let listed = vec![info(id(1, 3, "SER", "1-1")), info(id(9, 2, "SER", "1-1"))];
        assert!(locate(&wanted, &listed).is_err());
        assert!(locate(&wanted, &[]).is_err());
    }
}
