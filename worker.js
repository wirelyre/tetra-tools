importScripts("./pkg/solver.js");

async function main() {
    await wasm_bindgen("./pkg/solver_bg.wasm");
    let solver = new wasm_bindgen.Solver();

    console.log("ready");
    postMessage("ready");

    onmessage = message => {
        let pieces = message.data[0];
        let count = message.data[1];

        let solns;

        if (count == 0) {
            solns = solver.solve(pieces).split(",");
        } else {
            solns = solver.solve_some(pieces, count).split(",");
        }

        count = solns.shift();

        postMessage([pieces, count, solns]);
    }
}
main();
