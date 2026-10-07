---
document_type: behavioral-contract
version: "1.0"
bc_id: BC-9.99.012
---

# BC-9.99.012: fixture BC with an invalid UTF-8 byte sequence

This line contains an invalid UTF-8 continuation byte: ÿþ (lone 0xFF/0xFE, not a valid UTF-8 sequence).
