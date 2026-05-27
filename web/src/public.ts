import Ical from 'ical.js';
import { Calendar as FullCalendar } from '@fullcalendar/core';
import dayGridPlugin from '@fullcalendar/daygrid';
import timeGridPlugin from '@fullcalendar/timegrid';
import interactionPlugin from '@fullcalendar/interaction';

const calendarId = window.location.pathname.split('/').pop();
const icsUrl = `/api/calendars/${calendarId}/forked.ics`;

async function loadCalendar() {
  const res = await fetch(icsUrl);
  if (!res.ok) {
    document.getElementById('calendar-name')!.textContent = 'Calendar not found';
    return;
  }

  const text = await res.text();
  const jcalData = Ical.parse(text);
  const comp = new Ical.Component(jcalData);

  const nameProp = comp.getFirstProperty('name') || comp.getFirstProperty('x-wr-calname');
  const name = nameProp ? (nameProp.getFirstValue() as string) : 'Calendar';
  document.getElementById('calendar-name')!.textContent = name;

  const events = comp.getAllSubcomponents('vevent');

  const fcEvents = events.map((vevent) => {
    const event = new Ical.Event(vevent);
    return {
      id: event.uid,
      title: event.summary,
      start: event.startDate.toJSDate(),
      end: event.endDate.toJSDate(),
      location: event.location,
      description: event.description,
    };
  });

  const calendarEl = document.getElementById('calendar')!;
  const calendar = new FullCalendar(calendarEl, {
    plugins: [dayGridPlugin, timeGridPlugin, interactionPlugin],
    initialView: 'timeGridWeek',
    headerToolbar: {
      left: 'prev,next today',
      center: 'title',
      right: 'dayGridMonth,timeGridWeek,timeGridDay',
    },
    events: fcEvents,
    eventClick: (info) => {
      const parts = [info.event.title];
      if (info.event.extendedProps.location) parts.push(`📍 ${info.event.extendedProps.location}`);
      if (info.event.extendedProps.description) parts.push(info.event.extendedProps.description);
      alert(parts.join('\n'));
    },
  });
  calendar.render();
}

document.addEventListener('DOMContentLoaded', loadCalendar);
