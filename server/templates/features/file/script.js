const f = document.getElementById("f");
const text = document.querySelector("label[for='f']");
const b_a = document.getElementById("b_a");
const b_b = document.getElementById("b_b");
const a = document.getElementById("a");
const b = document.getElementById("b");
const link = document.getElementById("link");
const token = document.getElementById("token");
const yy = document.getElementById("yy");

f.addEventListener("change", function () {
    if (this.files.length > 0) {
        text.textContent = this.files[0].name;
    }
});

document.body.addEventListener("paste", (e) => {
    const data = e.clipboardData.items[0];
    if (data?.kind === "file" && data.type.startsWith("image/")) {
        const tf = new DataTransfer();
        tf.items.add(data.getAsFile());
        f.files = tf.files;
        text.textContent = "剪切板图片";
    }
});

b_a.addEventListener("click", function () {
    const file = f.files[0];
    if (!file) return;

    if (file.size > 5242880) {
        yy.textContent = "文件太大，必须小于 5mb";
        return;
    }

    const form = new FormData();
    form.append("f", file);

    a.classList.toggle("none");
    b.classList.remove("none");
    link.textContent = "正在加载...";
    token.textContent = "正在加载...";

    fetch(window.location.href, {
        method: "POST",
        body: form,
    })
        .then(response => response.json())
        .then(response => {
            link.textContent = response.link;
            token.textContent = response.token;
        });
});

b_b.addEventListener("click", function () {
    text.textContent = "选择文件";
    const tf = new DataTransfer();
    f.files = tf.files;

    a.classList.remove("none");
    b.classList.toggle("none");
});
