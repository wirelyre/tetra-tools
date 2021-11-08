importScripts("./pkg/solver.js");

async function main() {
    await wasm_bindgen("./pkg/solver_bg.wasm");
    let solver = new wasm_bindgen.Solver();

    console.log("ready");
    postMessage("ready");

    onmessage = message => {
        let [pieces, garbage, count] = message.data;
        let solns;

        if (!solver.possible(garbage)) {
            postMessage(["impossible", garbage]);
            return;
        }

        if (count == 0) {
            solns = solver.solve(pieces, garbage).split(",");
        } else {
            solns = solver.solve_some(pieces, garbage, count).split(",");
        }

        count = solns.shift();

        postMessage([pieces, garbage, count, solns]);
    }
}
main();
