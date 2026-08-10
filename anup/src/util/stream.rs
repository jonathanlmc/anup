use bytes::{BufMut, Bytes, BytesMut};
use futures::StreamExt;

pub async fn read_up_to_n_bytes<S, E>(
    stream: &mut S,
    max_bytes: usize,
    initial_capacity: usize,
) -> Result<LimitedReadResult<Bytes>, E>
where
    S: futures::Stream<Item = Result<Bytes, E>> + Unpin,
{
    let mut buffer = BytesMut::with_capacity(initial_capacity);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        // don't append the chunk if it would put us over the limit; bail early
        if chunk.len() + buffer.len() > max_bytes {
            return Ok(LimitedReadResult::Limited(buffer.freeze()));
        }

        buffer.put(chunk);
    }

    Ok(LimitedReadResult::UnderLimit(buffer.freeze()))
}

pub enum LimitedReadResult<T> {
    UnderLimit(T),
    Limited(T),
}
