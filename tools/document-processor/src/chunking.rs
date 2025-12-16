/// Utilities for splitting long documents into overlapping chunks
/// for grounded LLM extraction.

#[derive(Debug, Clone)]
pub struct TextChunk {
    pub index: usize,
    pub text: String,
}

/// Split text into overlapping chunks.
///
/// max_chars: maximum characters per chunk
/// overlap: number of characters to overlap between chunks
pub fn chunk_text(text: &str, max_chars: usize, overlap: usize) -> Vec<TextChunk> {
    let mut chunks = Vec::new();

    if text.is_empty() {
        return chunks;
    }

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut start = 0;
    let mut index = 0;

    while start < len {
        let end = usize::min(start + max_chars, len);

        let chunk_text: String = chars[start..end].iter().collect();

        chunks.push(TextChunk {
            index,
            text: chunk_text,
        });

        if end == len {
            break;
        }

        start = end.saturating_sub(overlap);
        index += 1;
    }

    chunks
}

