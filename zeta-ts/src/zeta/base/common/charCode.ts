/**
 * Inlined character codes for hot-path comparisons with `String.charCodeAt`.
 *
 * Keep this domain-neutral: add broadly reusable ASCII/Unicode boundaries,
 * not language grammar or editor feature identities.
 */
export const enum CharCode {
	// C0 control characters used by text buffers and parsers.
	/** U+0000 NULL (`\0`). */
	Null = 0,
	/** U+0008 BACKSPACE (`\b`). */
	Backspace = 8,
	/** U+0009 CHARACTER TABULATION (`\t`). */
	Tab = 9,
	/** U+000A LINE FEED (`\n`). */
	LineFeed = 10,
	/** U+000B LINE TABULATION (`\v`). */
	VerticalTab = 11,
	/** U+000C FORM FEED (`\f`). */
	FormFeed = 12,
	/** U+000D CARRIAGE RETURN (`\r`). */
	CarriageReturn = 13,

	// Printable ASCII whitespace and punctuation.
	/** U+0020 SPACE. */
	Space = 32,
	ExclamationMark = 33, // !
	DoubleQuote = 34, // "
	Hash = 35, // #
	DollarSign = 36, // $
	PercentSign = 37, // %
	Ampersand = 38, // &
	SingleQuote = 39, // '
	OpenParen = 40, // (
	CloseParen = 41, // )
	Asterisk = 42, // *
	Plus = 43, // +
	Comma = 44, // ,
	Dash = 45, // -
	Period = 46, // .
	Slash = 47, // /

	// ASCII decimal digits form one contiguous range.
	Digit0 = 48,
	Digit1 = 49,
	Digit2 = 50,
	Digit3 = 51,
	Digit4 = 52,
	Digit5 = 53,
	Digit6 = 54,
	Digit7 = 55,
	Digit8 = 56,
	Digit9 = 57,

	Colon = 58, // :
	Semicolon = 59, // ;
	LessThan = 60, // <
	Equals = 61, // =
	GreaterThan = 62, // >
	QuestionMark = 63, // ?
	AtSign = 64, // @

	// ASCII uppercase letters form one contiguous range.
	A = 65,
	B = 66,
	C = 67,
	D = 68,
	E = 69,
	F = 70,
	G = 71,
	H = 72,
	I = 73,
	J = 74,
	K = 75,
	L = 76,
	M = 77,
	N = 78,
	O = 79,
	P = 80,
	Q = 81,
	R = 82,
	S = 83,
	T = 84,
	U = 85,
	V = 86,
	W = 87,
	X = 88,
	Y = 89,
	Z = 90,

	OpenSquareBracket = 91, // [
	Backslash = 92, // \
	CloseSquareBracket = 93, // ]
	Caret = 94, // ^
	Underline = 95, // _
	BackTick = 96, // `

	// ASCII lowercase letters form one contiguous range.
	a = 97,
	b = 98,
	c = 99,
	d = 100,
	e = 101,
	f = 102,
	g = 103,
	h = 104,
	i = 105,
	j = 106,
	k = 107,
	l = 108,
	m = 109,
	n = 110,
	o = 111,
	p = 112,
	q = 113,
	r = 114,
	s = 115,
	t = 116,
	u = 117,
	v = 118,
	w = 119,
	x = 120,
	y = 121,
	z = 122,

	OpenCurlyBrace = 123, // {
	Pipe = 124, // |
	CloseCurlyBrace = 125, // }
	Tilde = 126, // ~
	/** U+007F DELETE control character. */
	Delete = 127,

	// Unicode whitespace and line-boundary characters relevant to text models.
	/** U+0085 NEXT LINE, a legacy Unicode line separator. */
	NextLine = 0x0085,
	/** U+00A0 NO-BREAK SPACE, visually space-like but non-breaking. */
	NoBreakSpace = 0x00A0,

	// Combining marks emitted by macOS/Linux dead-key mappings.
	/** U+0300 COMBINING GRAVE ACCENT. */
	CombiningGraveAccent = 0x0300,
	/** U+0301 COMBINING ACUTE ACCENT. */
	CombiningAcuteAccent = 0x0301,
	/** U+0302 COMBINING CIRCUMFLEX ACCENT. */
	CombiningCircumflexAccent = 0x0302,
	/** U+0303 COMBINING TILDE. */
	CombiningTilde = 0x0303,
	/** U+0304 COMBINING MACRON. */
	CombiningMacron = 0x0304,
	/** U+0306 COMBINING BREVE. */
	CombiningBreve = 0x0306,
	/** U+0307 COMBINING DOT ABOVE. */
	CombiningDotAbove = 0x0307,
	/** U+0308 COMBINING DIAERESIS. */
	CombiningDiaeresis = 0x0308,
	/** U+030A COMBINING RING ABOVE. */
	CombiningRingAbove = 0x030A,
	/** U+030B COMBINING DOUBLE ACUTE ACCENT. */
	CombiningDoubleAcuteAccent = 0x030B,
	/** U+030C COMBINING CARON. */
	CombiningCaron = 0x030C,
	/** U+0327 COMBINING CEDILLA. */
	CombiningCedilla = 0x0327,
	/** U+0328 COMBINING OGONEK. */
	CombiningOgonek = 0x0328,
	/** Last code point in the Combining Diacritical Marks block. */
	CombiningDiacriticalMarksEnd = 0x036F,
	/** U+200B ZERO WIDTH SPACE. */
	ZeroWidthSpace = 0x200B,
	/** U+2028 LINE SEPARATOR. */
	LineSeparator = 0x2028,
	/** U+2029 PARAGRAPH SEPARATOR. */
	ParagraphSeparator = 0x2029,

	// Inclusive UTF-16 surrogate ranges used when preserving code-point boundaries.
	/** First UTF-16 high-surrogate code unit. */
	HighSurrogateStart = 0xD800,
	/** Last UTF-16 high-surrogate code unit. */
	HighSurrogateEnd = 0xDBFF,
	/** First UTF-16 low-surrogate code unit. */
	LowSurrogateStart = 0xDC00,
	/** Last UTF-16 low-surrogate code unit. */
	LowSurrogateEnd = 0xDFFF,

	/** U+FEFF BYTE ORDER MARK when present at the beginning of decoded text. */
	ByteOrderMark = 0xFEFF,
	/** U+FFFD REPLACEMENT CHARACTER used for invalid or unknown input. */
	ReplacementCharacter = 0xFFFD,
}
