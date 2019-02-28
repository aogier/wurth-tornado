# Tornado Rsyslog Collector (executable)

The rsyslog Collector binary is an executable that generates Tornado Events from
rsyslog inputs.



## How It Works

This Collector is meant to be integrated with rsyslog’s own logging through the
[omprog module](https://www.rsyslog.com/doc/v8-stable/configuration/modules/omprog.html).
Consequently, it is never started manually, but instead will be started, and managed,
directly by rsyslog itself.

Here is an example rsyslog configuration template that pipes logs to the rsyslog-collector
(the parameters are explained below):
```
module(load="omprog")

action(type="omprog"
       binary="/path/to/tornado_rsyslog_collector --some-collector-options")
```

An example of a fully instantiated startup setup is:
```
module(load="omprog")

action(type="omprog"
       binary="/path/to/rsyslog_collector --logger-file-path=/log/rsys-collector.log --logger-level=info --uds-path=/tmp/tornado")
```

<!-- This part may only be necessary for non-expert users. Hide until later? -->
Note that all parameters for the _binary_ option must be on the same line. You will need to
place this configuration in a file in your rsyslog directory, for instance:
```
/etc/rsyslog.d/tornado.conf
```

In this example the collector will:
- Log to the file _/log/rsys-collector.log_ at the _info_ logger level
- Write outgoing Events to the UDS socket in _/tmp/tornado_  <!-- Isn't there more than one UDS socket there? -->

The Collector will need to be run in parallel with the Tornado engine before any events will be
processed, for example:  <!-- Link to the description of that executable -->
```
/opt/tornado/bin/tornado --logger-file-path=/tmp/my-tornado.log --uds-path /tmp/tornado/my-tornado.sock
```

<!-- Charles and Andrea had errors due to missing .toml files for the archive executor, which we assume will be fixed later. -->

Under this configuration, rsyslog is in charge of starting the collector when needed and piping
the incoming logs to it. As the last stage, the Tornado Events generated by the collector are
forwarded to the Tornado Engine's UDS socket.

This integration strategy is the best option for supporting high performance given massive
amounts of log data.

Because the collector expects the input to be in JSON format, **rsyslog should be pre-configured**
to properly pipe its inputs in this form.



## Configuration Options

This collector's configuration is based on the following command line parameters:
- __logger-stdout__:  Determines whether the Logger should print to standard output.
  Valid values are `true` and `false`, defaults to `false`.
- __logger-file-path__:  A file path in the file system; if provided, the Logger will
  append any output to it.
- __logger-level__:  The Logger level; valid values are _trace_, _debug_, _info_, _warn_, and
  _error_, defaulting to _warn_.
- __uds-path__:  The Unix Socket path where outgoing events will be written.
  This should be the path where Tornado Engine is listening for incoming events.
  By default it is _/var/run/tornado/tornado.sock_.
- __uds-mailbox-capacity__:  The in-memory buffer size for Events. It makes the application
  resilient to Tornado Engine crashes or temporary unavailability.
  When Tornado restarts, all messages in the buffer will be sent.
  When the buffer is full, the collector will start discarding old messages.
  The default buffer value is `10000`.

More information about the logger configuration is available
[here](../../../common/logger/doc/README.md).