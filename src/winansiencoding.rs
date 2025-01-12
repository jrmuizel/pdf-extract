#[derive(Debug, Clone)]
pub struct WinAnsiEncoding {
    mapping: [u16; 256],
}

impl Default for WinAnsiEncoding {
    fn default() -> Self {
        let mut mapping = [0u16; 256];

        // Standard ASCII range (0x20-0x7E)
        for i in 0x20..=0x7E {
            mapping[i] = i as u16;
        }

        // Special WinAnsi characters
        mapping[0x80] = 0x20AC; // €
        mapping[0x82] = 0x201A; // ‚
        mapping[0x83] = 0x0192; // ƒ
        mapping[0x84] = 0x201E; // „
        mapping[0x85] = 0x2026; // …
        mapping[0x86] = 0x2020; // †
        mapping[0x87] = 0x2021; // ‡
        mapping[0x88] = 0x02C6; // ˆ
        mapping[0x89] = 0x2030; // ‰
        mapping[0x8A] = 0x0160; // Š
        mapping[0x8B] = 0x2039; // ‹
        mapping[0x8C] = 0x0152; // Œ
        mapping[0x8E] = 0x017D; // Ž
        mapping[0x91] = 0x2018; // '
        mapping[0x92] = 0x2019; // '
        mapping[0x93] = 0x201C; // "
        mapping[0x94] = 0x201D; // "
        mapping[0x95] = 0x2022; // •
        mapping[0x96] = 0x2013; // –
        mapping[0x97] = 0x2014; // —
        mapping[0x98] = 0x02DC; // ˜
        mapping[0x99] = 0x2122; // ™
        mapping[0x9A] = 0x0161; // š
        mapping[0x9B] = 0x203A; // ›
        mapping[0x9C] = 0x0153; // œ
        mapping[0x9E] = 0x017E; // ž
        mapping[0x9F] = 0x0178; // Ÿ
        mapping[0xA0] = 0x00A0; // non-breaking space
        mapping[0xA1] = 0x00A1; // ¡
        mapping[0xA2] = 0x00A2; // ¢
        mapping[0xA3] = 0x00A3; // £
        mapping[0xA4] = 0x00A4; // ¤
        mapping[0xA5] = 0x00A5; // ¥
        mapping[0xA6] = 0x00A6; // ¦
        mapping[0xA7] = 0x00A7; // §
        mapping[0xA8] = 0x00A8; // ¨
        mapping[0xA9] = 0x00A9; // ©
        mapping[0xAA] = 0x00AA; // ª
        mapping[0xAB] = 0x00AB; // «
        mapping[0xAC] = 0x00AC; // ¬
        mapping[0xAD] = 0x00AD; // soft hyphen
        mapping[0xAE] = 0x00AE; // ®
        mapping[0xAF] = 0x00AF; // ¯
        mapping[0xB0] = 0x00B0; // °
        mapping[0xB1] = 0x00B1; // ±
        mapping[0xB2] = 0x00B2; // ²
        mapping[0xB3] = 0x00B3; // ³
        mapping[0xB4] = 0x00B4; // ´
        mapping[0xB5] = 0x00B5; // µ
        mapping[0xB6] = 0x00B6; // ¶
        mapping[0xB7] = 0x00B7; // ·
        mapping[0xB8] = 0x00B8; // ¸
        mapping[0xB9] = 0x00B9; // ¹
        mapping[0xBA] = 0x00BA; // º
        mapping[0xBB] = 0x00BB; // »
        mapping[0xBC] = 0x00BC; // ¼
        mapping[0xBD] = 0x00BD; // ½
        mapping[0xBE] = 0x00BE; // ¾
        mapping[0xBF] = 0x00BF; // ¿
        mapping[0xC0] = 0x00C0; // À
        mapping[0xC1] = 0x00C1; // Á
        mapping[0xC2] = 0x00C2; // Â
        mapping[0xC3] = 0x00C3; // Ã
        mapping[0xC4] = 0x00C4; // Ä
        mapping[0xC5] = 0x00C5; // Å
        mapping[0xC6] = 0x00C6; // Æ
        mapping[0xC7] = 0x00C7; // Ç
        mapping[0xC8] = 0x00C8; // È
        mapping[0xC9] = 0x00C9; // É
        mapping[0xCA] = 0x00CA; // Ê
        mapping[0xCB] = 0x00CB; // Ë
        mapping[0xCC] = 0x00CC; // Ì
        mapping[0xCD] = 0x00CD; // Í
        mapping[0xCE] = 0x00CE; // Î
        mapping[0xCF] = 0x00CF; // Ï
        mapping[0xD0] = 0x00D0; // Ð
        mapping[0xD1] = 0x00D1; // Ñ
        mapping[0xD2] = 0x00D2; // Ò
        mapping[0xD3] = 0x00D3; // Ó
        mapping[0xD4] = 0x00D4; // Ô
        mapping[0xD5] = 0x00D5; // Õ
        mapping[0xD6] = 0x00D6; // Ö
        mapping[0xD7] = 0x00D7; // ×
        mapping[0xD8] = 0x00D8; // Ø
        mapping[0xD9] = 0x00D9; // Ù
        mapping[0xDA] = 0x00DA; // Ú
        mapping[0xDB] = 0x00DB; // Û
        mapping[0xDC] = 0x00DC; // Ü
        mapping[0xDD] = 0x00DD; // Ý
        mapping[0xDE] = 0x00DE; // Þ
        mapping[0xDF] = 0x00DF; // ß
        mapping[0xE0] = 0x00E0; // à
        mapping[0xE1] = 0x00E1; // á
        mapping[0xE2] = 0x00E2; // â
        mapping[0xE3] = 0x00E3; // ã
        mapping[0xE4] = 0x00E4; // ä
        mapping[0xE5] = 0x00E5; // å
        mapping[0xE6] = 0x00E6; // æ
        mapping[0xE7] = 0x00E7; // ç
        mapping[0xE8] = 0x00E8; // è
        mapping[0xE9] = 0x00E9; // é
        mapping[0xEA] = 0x00EA; // ê
        mapping[0xEB] = 0x00EB; // ë
        mapping[0xEC] = 0x00EC; // ì
        mapping[0xED] = 0x00ED; // í
        mapping[0xEE] = 0x00EE; // î
        mapping[0xEF] = 0x00EF; // ï
        mapping[0xF0] = 0x00F0; // ð
        mapping[0xF1] = 0x00F1; // ñ
        mapping[0xF2] = 0x00F2; // ò
        mapping[0xF3] = 0x00F3; // ó
        mapping[0xF4] = 0x00F4; // ô
        mapping[0xF5] = 0x00F5; // õ
        mapping[0xF6] = 0x00F6; // ö
        mapping[0xF7] = 0x00F7; // ÷
        mapping[0xF8] = 0x00F8; // ø
        mapping[0xF9] = 0x00F9; // ù
        mapping[0xFA] = 0x00FA; // ú
        mapping[0xFB] = 0x00FB; // û
        mapping[0xFC] = 0x00FC; // ü
        mapping[0xFD] = 0x00FD; // ý
        mapping[0xFE] = 0x00FE; // þ
        mapping[0xFF] = 0x00FF; // ÿ

        WinAnsiEncoding { mapping }
    }
}

impl WinAnsiEncoding {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_slice(&self) -> &[u16] {
        &self.mapping
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::UTF_16BE;

    #[test]
    fn test_ascii_range() {
        let encoding = WinAnsiEncoding::default();
        assert_eq!(encoding.mapping[b'A' as usize], b'A' as u16);
        assert_eq!(encoding.mapping[b'z' as usize], b'z' as u16);
        assert_eq!(encoding.mapping[b'0' as usize], b'0' as u16);
    }

    #[test]
    fn test_special_characters() {
        let encoding = WinAnsiEncoding::default();
        assert_eq!(encoding.mapping[0x80], 0x20AC); // €
        assert_eq!(encoding.mapping[0x93], 0x201C); // "
        assert_eq!(encoding.mapping[0xA9], 0x00A9); // ©
    }

    #[test]
    fn test_to_utf8_conversion() {
        let encoding = WinAnsiEncoding::default();
        let test_chars = [(0x80, "€"), (0xA9, "©"), (0xE9, "é"), (0x41, "A")];

        for (input_byte, expected_char) in test_chars.iter() {
            let mapped = encoding.mapping[*input_byte as usize];
            let bytes = [(mapped >> 8) as u8, mapped as u8];

            let result = UTF_16BE
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .unwrap()
                .to_string();

            assert_eq!(&result, expected_char);
        }
    }

    #[test]
    fn test_undefined_characters() {
        let encoding = WinAnsiEncoding::default();
        assert_eq!(encoding.mapping[0x81], 0);
        assert_eq!(encoding.mapping[0x8D], 0);
        assert_eq!(encoding.mapping[0x8F], 0);
        assert_eq!(encoding.mapping[0x90], 0);
    }
}
