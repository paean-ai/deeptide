const preview = document.getElementById('preview');
const title = document.getElementById('sample-title');
const copy = document.getElementById('sample-copy');
const openLink = document.getElementById('open-link');
const buttons = [...document.querySelectorAll('#nav button')];

for (const button of buttons) {
  button.addEventListener('click', () => {
    for (const item of buttons) item.classList.toggle('active', item === button);
    preview.src = button.dataset.src;
    title.textContent = button.dataset.title;
    copy.textContent = button.dataset.copy;
    openLink.href = button.dataset.src;
  });
}
