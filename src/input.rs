/// Eight buttons so you'd need 8 bits but instead they
/// Look at the buttons as two columns with four buttons each
///
/// Up       Select
/// Down     Start
/// Left     B
/// Right    A
///
/// Thus you only need 6 bits

pub struct Input<'a> {
	memory: &'a [[u8;4]; 2]
}

fn is_pressed() {
	// 0 is pressed, 1 is not pressed
}