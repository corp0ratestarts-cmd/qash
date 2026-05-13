COQC ?= coqc
COQDEP ?= coqdep
VFILES := concat_injective.v lyapunov_decrease.v
VOFILES := $(VFILES:.v=.vo)

all: .depend $(VOFILES)
.depend: $(VFILES)
	$(COQDEP) -Q . QASH $(VFILES) > .depend
-include .depend
%.vo: %.v
	$(COQC) -Q . QASH $<
clean:
	rm -f *.vo *.vos *.vok *.glob .depend
.PHONY: all clean
