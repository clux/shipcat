document.addEventListener("DOMContentLoaded", function (event) {

  const tabs = document.querySelectorAll('.tabList__tabItem')
  const tabButtons = Array.prototype.slice.call(tabs, 0).map(function (tab) {
    return tab.querySelector('button')
  })
  const tabContent = Array.prototype.slice.call(tabButtons, 0).map(function (button) {
    return button.getAttribute('data-tab')
  })

  function showTab(id) {
    const toShow = tabContent.filter(function (tabId) { return tabId === id })
    const toHide = tabContent.filter(function (tabId) { return tabId !== id })

    toShow.forEach(function (id) {
      document.querySelector(`[data-tab="${id}"]`).classList.add('is-active')
      const el = document.getElementById(id)
      if (el) { el.style.display = 'block' }
    })

    toHide.forEach(function (id) {
      document.querySelector(`[data-tab="${id}"]`).classList.remove('is-active')
      const el = document.getElementById(id)
      if (el) { el.style.display = 'none' }
    })
  }

  tabButtons.forEach(function (button) {
    button.addEventListener('click', function () {
      const id = button.getAttribute('data-tab')
      const content = document.getElementById(id)
      showTab(id)
    })
  })

  showTab(tabButtons[0].getAttribute('data-tab'))
})
