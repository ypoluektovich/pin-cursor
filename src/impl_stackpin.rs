use async_std::io::{Cursor, Read, Seek, Write};
use stackpin::FromUnpinned;

use crate::PinCursor;

unsafe impl<T> FromUnpinned<Cursor<T>> for PinCursor<T>
    where T: Unpin,
          Cursor<T>: Write + Read + Seek
{
    type PinData = ();

    unsafe fn from_unpinned(src: Cursor<T>) -> (Self, Self::PinData) {
        (PinCursor::wrap(src), ())
    }

    unsafe fn on_pin(&mut self, _pin_data: Self::PinData) {
        // do nothing
    }
}
