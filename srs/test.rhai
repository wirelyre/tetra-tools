
fn dump_field(n, lines) {
	println("┌──────────┐");
	for row_backwards in 0..lines {
		print("│");
		let row = lines - row_backwards - 1;
		for col in 0..10 {
			let mask = 1 << (10 * row + col);
			if (n & mask) == 0 {
				print("·");
			} else {
				print("█");
			}
		}
		print("│\n");
	}
	println("└──────────┘");
}

// let field = -1;
let field = 477801267;
let x = 2;
let y = 1;
dump_field(field, 3);
dump_field(upper_shift(field, x, y), 3);
dump_field(lower_shift(field, x, y), 3);

// print("hello, world!");
// print(`...`);

// print(add(3, 4));
// println(4);
