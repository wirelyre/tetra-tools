class MinoBoard extends HTMLElement {

    static colors = {
        'background': '#F3F3ED',
        'shadow': '#E7E7E2',
        'regular': {
            'G': '#686868',
            'I': '#41AFDE',
            'J': '#1883BF',
            'L': '#EF9536',
            'O': '#F7D33E',
            'S': '#66C65C',
            'T': '#B451AC',
            'Z': '#EF624D',
        },
        'top': {
            'G': '#949494',
            'I': '#43D3FF',
            'J': '#1BA6F9',
            'L': '#FFBF60',
            'O': '#FFF952',
            'S': '#88EE86',
            'T': '#E56ADD',
            'Z': '#FF9484',
        },
    };

    static boardRegex = /^[\sGIJLOSTZ_]*$/;
    static minoRegex = /[GIJLOSTZ_]/g;
    static whitespace = /^\s*$/;

    constructor(field) {
        super();

        if (field) {
            this.setup(field);
        } else {
            document.addEventListener('DOMContentLoaded',
                (_event) => this.setup(this.textContent));
        }
    }

    setup(field) {
        this.innerHTML = '';

        if (!MinoBoard.boardRegex.test(field)) {
            const unknown = field.match(/[^\sGIJLOSTZ_]/)[0];
            this.innerText = 'Cannot draw field. Unknown character: ' + unknown;
            return;
        }

        this.field = [];

        for (const line of field.split('\n')) {
            if (MinoBoard.whitespace.test(line)) {
                continue;
            }

            let row = '';

            for (const mino of line.matchAll(MinoBoard.minoRegex)) {
                row += mino[0];

                if (row.length == 10) {
                    this.field.push(row);
                    row = '';
                }
            }

            if (row.length % 10 != 0) {
                row = row.padEnd(10, '_');
                this.field.push(row);
            }
        }

        this.setAttribute('data-field', this.field.join('|'));

        const width = 200;
        const height = 20 * (this.field.length + 2);

        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('width', width);
        svg.setAttribute('height', height);
        svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
        this.appendChild(svg);

        const getMino = (field, row, col) => field[row]?.charAt(col);

        function* minos(field) {
            for (let row = 0; row < field.length; row++) {
                for (let col = 0; col < 10; col++) {
                    const mino = getMino(field, row, col);

                    if (mino != '_') {
                        yield [row, col, mino];
                    }
                }
            }
        }

        function rect(x, y, width, height, fill) {
            const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
            rect.setAttribute('x', x);
            rect.setAttribute('y', y);
            rect.setAttribute('width', width);
            rect.setAttribute('height', height);
            rect.setAttribute('fill', fill);
            return rect;
        }

        svg.appendChild(rect(0, 0, '100%', '100%', MinoBoard.colors['background']));

        for (const [row, col, mino] of minos(this.field)) {
            const color = MinoBoard.colors['top'][mino];

            svg.appendChild(rect(20 * col + 5, 20 * (row + 2) + 7, 20, 20,
                MinoBoard.colors['shadow']));

            svg.appendChild(rect(20 * col, 20 * (row + 2) - 4, 20, 4, color));
        }

        for (const [row, col, mino] of minos(this.field)) {
            const color = MinoBoard.colors['regular'][mino];
            svg.appendChild(rect(20 * col, 20 * (row + 2), 20, 20, color));
        }
    }

}

customElements.define('mino-board', MinoBoard);
