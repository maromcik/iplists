/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{html,js,svelte}',
    '../templates/**/*.html',
    '../templates/**/*.jinja',
  ],
  safelist: [
      'text-blue-500',
  ],
  theme: {
    extend: {
      colors: {
        'muni-blue': '#0000DC',
      },
      fontFamily: {
        handwriting: ['"Pacifico"', 'cursive'],
      },
    },
  },
  plugins: [],

};