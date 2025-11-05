/// Makes graphics. Has 12 registers
/// 160x144 pixels
/// Pixels are grouped in 8x8 squares called tiles with a color index from 0 to 3
/// 20x14 tiles
/// The system supports 256 background tiles and 256 object tiles
/// You can only have 40 sprites per game and 10 a single line

/// The screen is just a viewport into the background map.
/// The background map is 32x32
struct Tile {
	// Every pixel is 2bit. There are eight pixels in a line. This means every line is 2bytes.
	color_indexes: [[u8; 2]; 8],
}

// I can choose my own palette for
struct Palette {
	colors: [u8; 4],
}

struct Tilemap<'a> {
	references: [&'a Tile; 256],
}


/// Window is a directly overlay. Typically is placed on the right or bottom
/// There is no translucency or transparency
///
/// For objects, they all have an Object Attribute Map (OAM)
/// Sprites can be up to 16 pixels in height
/// They have translucency
struct Layers<'a> {
	background: Tilemap<'a>,
	window: Tilemap<'a>,
	objects: u8,
}


// Timing
// CRT style graphics will do scan-line based rendering
