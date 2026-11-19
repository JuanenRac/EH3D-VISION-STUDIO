//! The report of a folder as a job that runs in a thread of its own, so that the window keeps answering while the pictures are read: it counts the pictures done, it can be cancelled
//! between two pictures and it hands the result over once. Nothing here knows the window or draws anything; `Window` starts it, asks it how far it is and writes the report.

use crate::batch::{check_detailed, Checked};
use crate::fiducial::Settings;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

/// What every picture of the folder is read and searched with.
#[derive(Debug, Clone, Copy)]
pub struct BatchParams {
    pub max_mib: u64,
    pub settings: Settings,
    pub threshold: Option<u8>,
    pub flatten_px: usize,
}

/// Reads the pictures one after the other, adding one to `done` after each, and stops before the next one when `cancel` is raised (the answer is then `None`).
pub fn run(
    files: &[PathBuf],
    params: &BatchParams,
    done: &AtomicUsize,
    cancel: &AtomicBool,
) -> Option<Vec<Checked>> {
    let mut items = Vec::with_capacity(files.len());
    for file in files {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        items.push(check_detailed(
            file,
            params.max_mib,
            &params.settings,
            params.threshold,
            params.flatten_px,
        ));
        done.fetch_add(1, Ordering::Relaxed);
    }
    Some(items)
}

/// Where a job is.
#[derive(Debug, Clone, PartialEq)]
pub enum Poll {
    /// Still reading: pictures done of the total.
    Running { done: usize, total: usize },
    /// It was cancelled and handed nothing over.
    Cancelled,
    /// Every picture was read.
    Finished(Vec<Checked>),
    /// The thread ended without an answer (it failed).
    Lost,
}

/// A report that is being made in a thread.
#[derive(Debug)]
pub struct BatchJob {
    total: usize,
    done: Arc<AtomicUsize>,
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Option<Vec<Checked>>>,
}

impl BatchJob {
    /// Starts reading `files` in a thread of its own. It fails only when the system cannot give a thread.
    pub fn start(files: Vec<PathBuf>, params: BatchParams) -> Result<BatchJob, String> {
        let done = Arc::new(AtomicUsize::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (send, result) = mpsc::channel();
        let total = files.len();
        let (d, c) = (Arc::clone(&done), Arc::clone(&cancel));
        std::thread::Builder::new()
            .name("eh3d-vision-batch".to_string())
            .spawn(move || {
                // the window may be gone by now: nobody to hand the result to is not an error
                let _ = send.send(run(&files, &params, &d, &c));
            })
            .map_err(|e| format!("thread_failed: {e}"))?;
        Ok(BatchJob {
            total,
            done,
            cancel,
            result,
        })
    }

    /// The number of pictures read so far and the number there are.
    pub fn progress(&self) -> (usize, usize) {
        (
            self.done.load(Ordering::Relaxed).min(self.total),
            self.total,
        )
    }

    /// Asks the job to stop before the next picture.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether the job was asked to stop.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Looks, without waiting, whether the job is over. The result is handed over once: after `Finished`, `Cancelled` or `Lost` the job has nothing more to give.
    pub fn poll(&self) -> Poll {
        match self.result.try_recv() {
            Ok(Some(items)) => Poll::Finished(items),
            Ok(None) => Poll::Cancelled,
            Err(mpsc::TryRecvError::Empty) => {
                let (done, total) = self.progress();
                Poll::Running { done, total }
            }
            Err(mpsc::TryRecvError::Disconnected) => Poll::Lost,
        }
    }
}

impl Drop for BatchJob {
    /// A job whose window is gone stops at the next picture.
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::image::Gray;
    use std::fs;
    use std::time::{Duration, Instant};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir =
                std::env::temp_dir().join(format!("eh3d-vision-job-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pgm(g: &Gray) -> Vec<u8> {
        let mut out = format!("P5\n{} {}\n255\n", g.w, g.h).into_bytes();
        out.extend_from_slice(&g.data);
        out
    }

    fn params() -> BatchParams {
        BatchParams {
            max_mib: 8,
            settings: Config::default().settings(),
            threshold: None,
            flatten_px: 0,
        }
    }

    /// `n` small pictures with one mark each.
    fn pictures(s: &Scratch, n: usize) -> Vec<PathBuf> {
        (0..n)
            .map(|i| {
                let mut g = Gray::blank(60, 60, 220).unwrap();
                g.disc(30.0, 30.0, 8.0, 30);
                let path = s.0.join(format!("p{i:02}.pgm"));
                fs::write(&path, pgm(&g)).unwrap();
                path
            })
            .collect()
    }

    fn wait(job: &BatchJob) -> Poll {
        let start = Instant::now();
        loop {
            match job.poll() {
                Poll::Running { .. } => {
                    assert!(
                        start.elapsed() < Duration::from_secs(60),
                        "the job never ended"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
                other => return other,
            }
        }
    }

    #[test]
    fn every_picture_is_read_and_counted() {
        let s = Scratch::new("run");
        let files = pictures(&s, 4);
        let done = AtomicUsize::new(0);
        let items = run(&files, &params(), &done, &AtomicBool::new(false)).unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(done.load(Ordering::Relaxed), 4);
        assert!(items
            .iter()
            .all(|c| c.row.marks == 1 && c.row.status == "ok"));
    }

    #[test]
    fn a_raised_flag_stops_before_the_next_picture() {
        let s = Scratch::new("stop");
        let files = pictures(&s, 3);
        let done = AtomicUsize::new(0);
        assert!(run(&files, &params(), &done, &AtomicBool::new(true)).is_none());
        assert_eq!(done.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_picture_that_cannot_be_used_does_not_stop_the_others() {
        let s = Scratch::new("bad");
        let mut files = pictures(&s, 2);
        let bad = s.0.join("bad.pgm");
        fs::write(&bad, b"not a picture").unwrap();
        files.insert(1, bad);
        let done = AtomicUsize::new(0);
        let items = run(&files, &params(), &done, &AtomicBool::new(false)).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[1].row.status, "picture_unreadable");
        assert_eq!(items[2].row.marks, 1);
    }

    #[test]
    fn a_job_hands_its_result_over_once_and_reports_its_progress() {
        let s = Scratch::new("job");
        let job = BatchJob::start(pictures(&s, 5), params()).unwrap();
        assert_eq!(job.progress().1, 5);
        match wait(&job) {
            Poll::Finished(items) => assert_eq!(items.len(), 5),
            other => panic!("{other:?}"),
        }
        assert_eq!(job.progress(), (5, 5));
        // the answer was handed over: the sender is gone
        assert_eq!(job.poll(), Poll::Lost);
    }

    #[test]
    fn a_cancelled_job_ends_without_a_result() {
        let s = Scratch::new("cancel");
        let job = BatchJob::start(pictures(&s, 3), params()).unwrap();
        // raised at once: the thread may have read a picture or two before it looks, never more than there are
        job.cancel();
        assert!(job.is_cancelled());
        match wait(&job) {
            Poll::Cancelled => assert!(job.progress().0 <= 3),
            // the thread was faster than the flag: it finished all of them, which is also a whole answer
            Poll::Finished(items) => assert_eq!(items.len(), 3),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_job_over_no_pictures_finishes_at_once() {
        let job = BatchJob::start(Vec::new(), params()).unwrap();
        match wait(&job) {
            Poll::Finished(items) => assert!(items.is_empty()),
            other => panic!("{other:?}"),
        }
        assert_eq!(job.progress(), (0, 0));
    }
}
