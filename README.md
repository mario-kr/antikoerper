Antikörper
==========


Antikörper is meant to be a lightweight data aggregation and visualization tool.
It will have two main components:

- A Server, accepting/asking for data, saving and analyzing it.
    It will have these features:
    - Event Triggers, upon predefined Rules and based on the incoming data a
        specified action can be triggered. For example: "If the errorlog has
        another occurence of the word critical then send an email to the senior
        engineer."
    - Data Visualization, you will be able to view and look at the data that has
      been aggregated, allowing you view trends and changes over time.
    - Automatic addition of individual items, you can define rules that
      themselves add rules. For example you would like to have all your hard
      disks periodically queried for their SMART Data. You will be able to hot
      plug in drives and they will be automatically queried.
