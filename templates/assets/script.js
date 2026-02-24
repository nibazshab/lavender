const con = document.getElementById('con');
let tmp = con.value, timer;

con.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
        if (con.value === tmp) return;

        fetch(location.href, {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=utf-8' },
            body: new URLSearchParams({ t: con.value })
        }).then(r => r.ok && (tmp = con.value));
    }, 500);
});
