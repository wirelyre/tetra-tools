importScripts("./pkg/solver.js");

async function main() {
    await wasm_bindgen("./pkg/solver_bg.wasm");
    let solver = new wasm_bindgen.Solver();

    console.log("ready");
    postMessage({ kind: "ready" });

    onmessage = message => {
        let query = message.data;
        let solutions;

        if (!solver.possible(query.garbage)) {
            postMessage({ kind: "impossible", query });
            return;
        }

        let queue = new wasm_bindgen.Queue();

        let bag = /[ILJOSTZ]|\[([ILJOSTZ]+)\](\d*)|(\*)(\d*)/g;
        for (let match of query.queue.matchAll(bag)) {
            if (match[1]) {
                let count = parseInt(match[2], 10) || 1;
                queue.add_bag(match[1], count);
            } else if (match[3]) {
                let count = parseInt(match[4], 10) || 1;
                queue.add_bag("IJLOSTZ", count);
            } else {
                queue.add_shape(match[0]);
            }
        }

        if (query.count == 0) {
            solutions = solver.solve(queue, query.garbage, query.hold).split(",");
        } else {
            solutions = solver.solve_some(queue, query.garbage, query.hold, query.count).split(",");
        }

        count = solutions.shift();

        postMessage({ kind: "ok", query, solutions, count });
    }
}
main();
