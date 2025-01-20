const textarea = document.getElementById('con');
let old_con = textarea.value;
let life;

textarea.addEventListener('input', () => {
    clearTimeout(life);
    life = setTimeout(sendmsg, 500);
});

function sendmsg() {
    const new_con = textarea.value;
    if (new_con !== old_con) {
        fetch(window.location.href, {
            method: 'POST',
            body: new URLSearchParams({
                t: new_con
            })
        });
        old_con = new_con;
    }
}

document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key === 'r') {
        e.preventDefault();
        const re = document.getElementById('re');
        re.style.display = re.style.display === 'block' ? 'none' : 'block';
        if (re.style.display === 'block') {
            document.getElementById('tf').focus();
        }
    }
});

document.getElementById('b_re').addEventListener('click', () => {
    const tf = document.getElementById('tf').value;
    const tr = document.getElementById('tr').value;
    textarea.value = textarea.value.split(tf).join(tr);
    sendmsg()
});
