// Notifications and Toast Handling

window.showToast = function(message) {
    // Remove existing toast if any
    const existing = document.getElementById('runner-toast');
    if (existing) {
        existing.remove();
    }

    const toastDiv = document.createElement('div');
    toastDiv.id = 'runner-toast';
    toastDiv.className = 'fixed right-4 top-4 z-50 max-w-sm rounded border border-gray-200 bg-white px-4 py-3 shadow-lg';

    const textNode = document.createElement('p');
    textNode.className = 'text-sm font-semibold text-gray-900';
    textNode.textContent = message;

    toastDiv.appendChild(textNode);
    document.body.appendChild(toastDiv);

    setTimeout(() => {
        const t = document.getElementById('runner-toast');
        if (t) t.remove();
    }, 4000);
};

window.redirectToDashboard = function(message) {
    if (message) {
        window.location.replace('/?toast=' + encodeURIComponent(message));
    } else {
        window.location.replace('/');
    }
};

window.onload = function() {
    const params = new URLSearchParams(window.location.search);
    const toastMsg = params.get('toast');
    if (toastMsg) {
        window.showToast(toastMsg);

        // Remove toast from URL without refreshing the page
        const newUrl = window.location.protocol + "//" + window.location.host + window.location.pathname;
        window.history.replaceState({path:newUrl},'',newUrl);
    }
};
