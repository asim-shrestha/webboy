/// Four voices with 4 bytes each: Control, Frequency, Volume, Length, Sweep
/// - Pulse A: Only the first pulse voice has the concent of a frequency sweep
/// - Pulse B:
/// - Pulse Wave:
/// - Noise: 4th pulse register that can only do noise
/// All voices have a trigger bit

struct Registers {

	wave: None,
}

/// Wave register has 16 extra bytes to represent different wave forms