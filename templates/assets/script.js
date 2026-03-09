const con = document.getElementById('con');
let tmp = con.value, timer;

con.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
        if (con.value === tmp) return;

        fetch(location.href, {
            method: 'POST',
            headers: { 'Content-Type': 'text/plain;charset=utf-8' },
            body: con.value
        }).then(r => r.ok && (tmp = con.value));
    }, 500);
});
