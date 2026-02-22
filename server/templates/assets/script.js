(() => {
    const con = document.getElementById('con');
    if (!con) {
        return;
    }

    let con_a = con.value;
    const send = () => {
        const con_b = con.value;
        if (con_b === con_a) return;

        fetch(window.location.href, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded; charset=utf-8',
            },
            body: new URLSearchParams({t: con_b})
        }).then(r => {
            if (r.ok) {
                con_a = con_b;
            }
        })
    };

    let timer;
    con.addEventListener('input', () => {
        clearTimeout(timer);
        timer = setTimeout(send, 500);
    });

    const replace = () => {
        const re = document.createElement('div');
        re.className = 're';

        const re_a = document.createElement('input');
        re_a.type = 'text';
        re_a.placeholder = 'a';

        const re_b = document.createElement('input');
        re_b.type = 'text';
        re_b.placeholder = 'b';

        const re_r = document.createElement('button');
        re_r.textContent = 'run';

        re_r.addEventListener('click', () => {
            const a = re_a.value;
            const b = re_b.value;

            if (a) {
                con.value = con.value.replaceAll(a, b);
                send();
            }
        });

        re.append(re_a, re_b, re_r);
        document.body.appendChild(re);

        return {re, re_a};
    };

    let widget = null;
    document.addEventListener('keydown', (e) => {
        if (e.ctrlKey && e.key.toLowerCase() === 'r') {
            e.preventDefault();
            if (!widget) {
                widget = replace();
            }

            const {re, re_a} = widget;
            re.classList.toggle('visible');

            if (re.classList.contains('visible')) {
                re_a.focus();
                re_a.select();
            }
        }

        if (e.key === 'Escape') {
            widget?.re.classList.remove('visible');
        }
    });
})();
