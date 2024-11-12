import {Component, html, decorator} from 'veda-client';

class Spinner extends Component(HTMLElement) {
  static tag = 'bpa-spinner';

  show() {
    this.lastElementChild.style.display = '';
  }

  hide() {
    this.lastElementChild.style.display = 'none';
  }
  render() {
    return html`
      <style>
        ${Spinner} .overlay {
          position: fixed;
          z-index: 1000;
          top: 0;
          left: 0;
          bottom: 0;
          right: 0;
          opacity: 0.5;
          background-color: white;
          filter: alpha(opacity=50);
        }
        ${Spinner} .spinner-container {
          position: fixed;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
        }
      </style>
      <div class="overlay" style="display: none;">
        <div class="spinner-container">
          <div class="spinner-border text-primary" role="status" style="width: 3rem; height: 3rem;">
            <span class="visually-hidden">Loading...</span>
          </div>
        </div>
      </div>
    ` 
  }
}

customElements.define(Spinner.tag, Spinner);

const spinner = document.createElement(`${Spinner}`);
document.body.appendChild(spinner);

function showSpinner () {
  spinner.show();
}

function hideSpinner () {
  spinner.hide();
}

function errorHandler (error) {
  hideSpinner();
  console.error(error);
  throw error;
}

export default function spinnerDecorator (fn, pre = showSpinner, post = hideSpinner, err = errorHandler) {
  return decorator(fn, pre, post, err);
}
