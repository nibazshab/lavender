const con = document.getElementById('con');
let prev = con.value, timer;

con.addEventListener('input', () => {
    clearTimeout(timer);

    timer = setTimeout(() => {
        if (con.value === prev) return;

        fetch(location.href, {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=utf-8' },
            body: new URLSearchParams({ t: con.value })
        }).then(r => r.ok && (prev = con.value));
    }, 500);
});
